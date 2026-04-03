#![no_std]
extern crate alloc;

use aidoku::{
    error::Result,
    helpers::uri::encode_uri,
    prelude::*,
    std::{
        net::{HttpMethod, Request},
        String, Vec,
    },
    Chapter, Filter, FilterType, Manga, MangaContentRating, MangaPageResult, MangaStatus,
    MangaViewer, Page, Listing,
};
use alloc::string::ToString;

const WWW_URL: &str = "https://nnhanman.xyz";
const UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1";

fn extract_manga_id(href: &str) -> String {
    href.split("/comic/")
        .last()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("")
        .replace(".html", "")
}

fn absolute_url(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if path.starts_with("//") {
        format!("https:{}", path)
    } else {
        format!("{}{}", WWW_URL, path)
    }
}

fn looks_like_image(url: &str) -> bool {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("yandex.ru/watch")
        || lower.contains("/images/logo")
        || lower.ends_with("/logo.png")
        || lower.ends_with("favicon.ico")
    {
        return false;
    }
    lower.contains(".jpg")
        || lower.contains(".jpeg")
        || lower.contains(".png")
        || lower.contains(".webp")
        || lower.contains(".gif")
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
    let mut query = String::new();

    for filter in filters {
        if filter.kind == FilterType::Title {
            query = filter.value.as_string()?.read();
        }
    }

    let url = if query.is_empty() {
        if page > 1 {
            return Ok(MangaPageResult {
                manga: Vec::new(),
                has_more: false,
            });
        }
        format!("{}/update", WWW_URL)
    } else {
        format!("{}/search/{}/page/{}", WWW_URL, encode_uri(query.clone()), page)
    };

    let html = Request::new(url, HttpMethod::Get)
        .header("User-Agent", UA)
        .html()?;

    let mut mangas: Vec<Manga> = Vec::new();

    for item in html
        .select(".UpdateList .itemBox, .SearchList .itemBox, .itemBox")
        .array()
    {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let mut href = item.select(".itemImg a").attr("href").read();
        if href.is_empty() {
            href = item.select("a.title").attr("href").read();
        }
        if href.is_empty() {
            href = item.select("a").attr("href").read();
        }
        if !href.contains("/comic/") {
            continue;
        }

        let id = extract_manga_id(href.as_str());
        if id.is_empty() {
            continue;
        }

        let image = item.select(".itemImg img, img");
        let mut cover = image.attr("src").read();
        if cover.is_empty() {
            cover = image.attr("data-src").read();
        }
        if cover.is_empty() {
            cover = image.attr("data-original").read();
        }
        if !cover.is_empty() {
            cover = absolute_url(cover.as_str());
        }

        let mut title = item.select(".itemImg a").attr("title").read();
        if title.is_empty() {
            title = item.select("a.title").text().read();
        }
        if title.is_empty() {
            title = image.attr("alt").read();
        }
        if title.is_empty() {
            continue;
        }

        mangas.push(Manga {
            id,
            title,
            cover,
            ..Default::default()
        });
    }

    let has_more = if query.is_empty() {
        false
    } else {
        html.html()
            .read()
            .contains(format!("/page/{}", page + 1).as_str())
    };

    Ok(MangaPageResult {
        manga: mangas,
        has_more,
    })
}

#[get_manga_listing]
fn get_manga_listing(_: Listing, page: i32) -> Result<MangaPageResult> {
    get_manga_list(Vec::new(), page)
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
    let url = format!("{}/comic/{}.html", WWW_URL, id.clone());
    let html = Request::new(url.clone(), HttpMethod::Get)
        .header("User-Agent", UA)
        .html()?;

    let cover = html.select(".Introduct_Sub .pic img").attr("src").read();
    let title = html.select(".Introduct_Sub h1").text().read();

    let author_nodes = html.select(".Introduct_Sub .sub_r .txtItme").array();
    let author = if author_nodes.len() > 0 {
        match author_nodes.get(0).as_node() {
            Ok(node) => node.text().read().trim().to_string(),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    let categories = html
        .select(".Introduct_Sub .sub_r .txtItme a")
        .array()
        .map(|x| x.as_node().unwrap().text().read())
        .collect::<Vec<String>>();

    let description = html.select(".txtDesc").text().read().trim().to_string();

    Ok(Manga {
        id,
        title,
        cover,
        author,
        artist: String::new(),
        description,
        url,
        categories,
        status: MangaStatus::Unknown,
        nsfw: MangaContentRating::Nsfw,
        viewer: MangaViewer::Scroll,
    })
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
    let url = format!("{}/comic/{}.html", WWW_URL, id.clone());
    let html = Request::new(url, HttpMethod::Get)
        .header("User-Agent", UA)
        .html()?;

    let list = html
        .select("a[href*='/comic/'][href*='/chapter-']")
        .array();
    let len = list.len();
    let mut chapters: Vec<Chapter> = Vec::new();

    for (index, item) in list.enumerate() {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let href = item.attr("href").read();
        if !href.contains("/comic/") || !href.contains("/chapter-") {
            continue;
        }

        let chapter_id = href.trim_start_matches('/').to_string();
        if chapters.iter().any(|x| x.id == chapter_id) {
            continue;
        }
        let mut title = item.attr("title").read().trim().to_string();
        if title.is_empty() {
            title = item.text().read().trim().to_string();
        }
        let chapter = (len - index) as f32;

        chapters.push(Chapter {
            id: chapter_id.clone(),
            title,
            chapter,
            url: absolute_url(format!("/{}", chapter_id).as_str()),
            ..Default::default()
        });
    }

    Ok(chapters)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
    let chapter_url = if chapter_id.starts_with("http") {
        chapter_id
    } else if chapter_id.starts_with('/') {
        absolute_url(chapter_id.as_str())
    } else {
        absolute_url(format!("/{}", chapter_id).as_str())
    };

    let html = Request::new(chapter_url, HttpMethod::Get)
        .header("User-Agent", UA)
        .html()?;

    let mut pages: Vec<Page> = Vec::new();

    for (index, item) in html
        .select("img.lazy, .comic-cont img, .read-content img, article img, .content img, img")
        .array()
        .enumerate()
    {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let mut url = item.attr("data-original").read();
        if url.is_empty() {
            url = item.attr("data-src").read();
        }
        if url.is_empty() {
            url = item.attr("src").read();
        }

        if !looks_like_image(url.as_str()) {
            continue;
        }

        pages.push(Page {
            index: index as i32,
            url,
            ..Default::default()
        });
    }

    Ok(pages)
}

#[modify_image_request]
fn modify_image_request(request: Request) {
    let _ = request.header("User-Agent", UA).header("Referer", WWW_URL);
}

