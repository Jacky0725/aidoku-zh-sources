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

const WWW_URL: &str = "https://kxmanhua.com";
const UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1";

fn parse_manga_id(href: &str) -> String {
    href.split("/manga/")
        .last()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

fn to_absolute(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{}{}", WWW_URL, url)
    }
}

fn parse_manga_cards(html: aidoku::std::html::Node) -> Vec<Manga> {
    let mut list: Vec<Manga> = Vec::new();

    for item in html.select(".product__item").array() {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let link = item.select("h6 a");
        let href = link.attr("href").read();
        if !href.contains("/manga/") {
            continue;
        }

        let id = parse_manga_id(href.as_str());
        if id.is_empty() {
            continue;
        }

        let title = link.text().read();
        if title.is_empty() {
            continue;
        }

        let mut cover = item.select(".product__item__pic").attr("data-setbg").read();
        if cover.is_empty() {
            cover = item.select("img").attr("src").read();
        }

        list.push(Manga {
            id,
            title,
            cover,
            ..Default::default()
        });
    }

    list
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
        format!(
            "{}/manga/library?type=0&complete=1&page={}&orderby=1",
            WWW_URL, page
        )
    } else {
        format!(
            "{}/manga/search?keyword={}&page={}",
            WWW_URL,
            encode_uri(query),
            page
        )
    };

    let html = Request::new(url, HttpMethod::Get)
        .header("User-Agent", UA)
        .header("Accept-Encoding", "identity")
        .html()?;
    let page_html = html.html().read();

    let mangas = parse_manga_cards(html);
    let has_more = page_html.contains(format!("page={}", page + 1).as_str());

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
    let url = format!("{}/manga/{}", WWW_URL, id.clone());
    let html = Request::new(url.clone(), HttpMethod::Get)
        .header("User-Agent", UA)
        .header("Accept-Encoding", "identity")
        .html()?;

    let title = html.select(".anime__details__title h3").text().read();
    let cover = html.select(".anime__details__pic").attr("data-setbg").read();

    let author_nodes = html.select(".anime__details__title span").array();
    let author = if author_nodes.len() > 0 {
        match author_nodes.get(0).as_node() {
            Ok(node) => node
                .text()
                .read()
                .replace("\u{4f5c}\u{8005}\u{ff1a}", "")
                .replace("\u{4f5c}\u{8005}:", "")
                .trim()
                .to_string(),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    let categories = html
        .select(".anime__details__widget a")
        .array()
        .map(|x| x.as_node().unwrap().text().read())
        .collect::<Vec<String>>();

    let status_text = html
        .select(".anime__details__pic .ep, .anime__details__pic .epgreen")
        .text()
        .read();
    let status = if status_text.contains("\u{8fde}\u{8f7d}") || status_text.to_ascii_lowercase().contains("ongoing") {
        MangaStatus::Ongoing
    } else if status_text.contains("\u{5b8c}\u{7ed3}") || status_text.to_ascii_lowercase().contains("completed") {
        MangaStatus::Completed
    } else {
        MangaStatus::Unknown
    };

    let mut description = html
        .select(".anime__details__text p")
        .array()
        .map(|x| x.as_node().unwrap().text().read())
        .find(|x| !x.trim().is_empty())
        .unwrap_or_default();

    if description.trim().is_empty() {
        description = html.select("meta[name='description']").attr("content").read();
    }

    Ok(Manga {
        id,
        title,
        cover,
        author,
        artist: String::new(),
        description,
        url,
        categories,
        status,
        nsfw: MangaContentRating::Nsfw,
        viewer: MangaViewer::Scroll,
    })
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
    let url = format!("{}/manga/{}", WWW_URL, id.clone());
    let html = Request::new(url, HttpMethod::Get)
        .header("User-Agent", UA)
        .header("Accept-Encoding", "identity")
        .html()?;

    let links = html.select(".chapter_list a, .anime__details__episodes a").array();
    let len = links.len();
    let mut chapters: Vec<Chapter> = Vec::new();

    for (index, item) in links.enumerate() {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let href = item.attr("href").read();
        if !href.contains("/manga/") || !href.contains("/detail/") {
            continue;
        }

        let title = item.text().read().trim().to_string();
        let chapter = (len - index) as f32;

        chapters.push(Chapter {
            id: href.clone(),
            title,
            chapter,
            url: to_absolute(href.as_str()),
            ..Default::default()
        });
    }

    Ok(chapters)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
    let url = if chapter_id.starts_with("http") {
        chapter_id
    } else {
        to_absolute(chapter_id.as_str())
    };

    let html = Request::new(url, HttpMethod::Get)
        .header("User-Agent", UA)
        .header("Accept-Encoding", "identity")
        .html()?;

    let mut pages: Vec<Page> = Vec::new();

    for (index, item) in html.select(".blog__details__content img").array().enumerate() {
        let item = match item.as_node() {
            Ok(node) => node,
            Err(_) => continue,
        };

        let src = item.attr("src").read();
        if src.starts_with("http") && src.contains("/webtoon/content/") {
            pages.push(Page {
                index: index as i32,
                url: src,
                ..Default::default()
            });
        }
    }

    Ok(pages)
}

#[modify_image_request]
fn modify_image_request(request: Request) {
    let _ = request.header("User-Agent", UA).header("Referer", WWW_URL);
}

