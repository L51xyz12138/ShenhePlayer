use crate::emby::{BaseItem, ItemsQuery, QueryResult, LIST_FIELDS};
use crate::error::Result;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSection {
    pub id: String,
    pub title: String,
    /// resume | nextup | latest
    pub kind: String,
    pub parent_id: Option<String>,
    pub items: Vec<BaseItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeData {
    pub views: Vec<BaseItem>,
    pub sections: Vec<HomeSection>,
    /// 顶部大图轮播用的条目
    pub hero: Vec<BaseItem>,
}

#[tauri::command]
pub async fn get_views(state: State<'_, Arc<AppState>>) -> Result<Vec<BaseItem>> {
    let client = state.client()?;
    Ok(client.views().await?.items)
}

/// 首页数据一次拿全：并发请求，避免前端串行等待
#[tauri::command]
pub async fn get_home(state: State<'_, Arc<AppState>>) -> Result<HomeData> {
    let client = state.client()?;

    let (views, resume, next_up) = tokio::join!(
        client.views(),
        client.resume(16),
        client.next_up(16),
    );

    let views = views?.items;
    let resume = resume.map(|r| r.items).unwrap_or_default();
    let next_up = next_up.map(|r| r.items).unwrap_or_default();

    let mut sections = Vec::new();
    if !resume.is_empty() {
        sections.push(HomeSection {
            id: "resume".into(),
            title: "继续观看".into(),
            kind: "resume".into(),
            parent_id: None,
            items: resume.clone(),
        });
    }
    if !next_up.is_empty() {
        sections.push(HomeSection {
            id: "nextup".into(),
            title: "接着看下一集".into(),
            kind: "nextup".into(),
            parent_id: None,
            items: next_up,
        });
    }

    // 每个媒体库的「最新添加」真正并发拉取：每个请求一个 task
    let media_views: Vec<(String, String)> = views
        .iter()
        .filter(|v| {
            matches!(
                v.collection_type.as_deref(),
                Some("movies") | Some("tvshows") | Some("music") | Some("homevideos") | None
            )
        })
        .take(6)
        .map(|v| (v.id.clone(), v.name.clone()))
        .collect();

    let handles: Vec<_> = media_views
        .iter()
        .map(|(id, _)| {
            let client = client.clone();
            let id = id.clone();
            tauri::async_runtime::spawn(async move { client.latest(Some(&id), 16).await })
        })
        .collect();

    for ((id, name), handle) in media_views.iter().zip(handles) {
        match handle.await {
            Ok(Ok(items)) if !items.is_empty() => sections.push(HomeSection {
                id: format!("latest-{id}"),
                title: format!("{name} · 最新添加"),
                kind: "latest".into(),
                parent_id: Some(id.clone()),
                items,
            }),
            Ok(Ok(_)) => {}
            Ok(Err(e)) => log::warn!("获取 {name} 最新添加失败: {e}"),
            Err(e) => log::warn!("任务失败: {e}"),
        }
    }

    // 顶部大图：优先用「继续观看」，否则用最新添加里有背景图的
    let mut hero: Vec<BaseItem> = resume
        .iter()
        .filter(|i| !i.backdrop_image_tags.is_empty() || i.parent_backdrop_item_id.is_some())
        .take(5)
        .cloned()
        .collect();
    if hero.len() < 5 {
        for s in sections.iter().filter(|s| s.kind == "latest") {
            for item in &s.items {
                if hero.len() >= 5 {
                    break;
                }
                if !item.backdrop_image_tags.is_empty()
                    && !hero.iter().any(|h| h.id == item.id)
                {
                    hero.push(item.clone());
                }
            }
        }
    }

    Ok(HomeData { views, sections, hero })
}

#[tauri::command]
pub async fn get_items(
    state: State<'_, Arc<AppState>>,
    mut query: ItemsQuery,
) -> Result<QueryResult<BaseItem>> {
    let client = state.client()?;
    if query.fields.is_none() {
        query.fields = Some(LIST_FIELDS.into());
    }
    if query.enable_image_types.is_none() {
        query.enable_image_types = Some("Primary,Backdrop,Thumb,Logo".into());
    }
    client.items(&query).await
}

#[tauri::command]
pub async fn get_item(state: State<'_, Arc<AppState>>, item_id: String) -> Result<BaseItem> {
    state.client()?.item(&item_id).await
}

#[tauri::command]
pub async fn get_seasons(
    state: State<'_, Arc<AppState>>,
    series_id: String,
) -> Result<Vec<BaseItem>> {
    Ok(state.client()?.seasons(&series_id).await?.items)
}

#[tauri::command]
pub async fn get_episodes(
    state: State<'_, Arc<AppState>>,
    series_id: String,
    season_id: Option<String>,
) -> Result<Vec<BaseItem>> {
    Ok(state
        .client()?
        .episodes(&series_id, season_id.as_deref())
        .await?
        .items)
}

#[tauri::command]
pub async fn get_similar(
    state: State<'_, Arc<AppState>>,
    item_id: String,
    limit: Option<u32>,
) -> Result<Vec<BaseItem>> {
    Ok(state
        .client()?
        .similar(&item_id, limit.unwrap_or(12))
        .await?
        .items)
}

#[tauri::command]
pub async fn search(
    state: State<'_, Arc<AppState>>,
    term: String,
    limit: Option<u32>,
) -> Result<Vec<BaseItem>> {
    if term.trim().is_empty() {
        return Ok(Vec::new());
    }
    let query = ItemsQuery {
        search_term: Some(term),
        include_item_types: Some("Movie,Series,Episode,BoxSet,Person".into()),
        recursive: Some(true),
        limit: Some(limit.unwrap_or(40)),
        fields: Some(LIST_FIELDS.into()),
        enable_image_types: Some("Primary,Backdrop,Thumb".into()),
        ..Default::default()
    };
    Ok(state.client()?.items(&query).await?.items)
}

#[tauri::command]
pub async fn set_favorite(
    state: State<'_, Arc<AppState>>,
    item_id: String,
    favorite: bool,
) -> Result<()> {
    state.client()?.set_favorite(&item_id, favorite).await
}

#[tauri::command]
pub async fn set_played(
    state: State<'_, Arc<AppState>>,
    item_id: String,
    played: bool,
) -> Result<()> {
    state.client()?.set_played(&item_id, played).await
}
