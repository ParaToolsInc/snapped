use std::path::Path;

use crate::{tbon::Tbon, utils::TbonHistogram};
use anyhow::Result;
use rouille::{Request, Response, Server};

/// Parse a URL and return a tuple containing the prefix and resource.
///
/// The input `surl` is expected to be a string starting with a forward slash `/`.
/// The function extracts the path components from the URL and returns them as a tuple.
///
/// # Examples
///
/// * A simple URL: `/metric`
///   - Returns: (`"metric"`, `""`)
/// * An empty URL: `/`
///   - Returns: (`"/"`, `""`)
/// * A complex URL: `/path/to/resource`
///   - Returns: (`"path/to/"`, `"resource"`)
///
fn parse_url(surl: &str) -> (String, String) {
    // Ensure the input string starts with a forward slash.
    assert!(surl.starts_with('/'));

    // Extract the URL path components by creating a `Path` instance.
    let url = surl[1..].to_string();
    let path = Path::new(&url);

    // Collect the path segments into a vector of strings.
    let segments: Vec<String> = path
        .iter()
        .map(|v| v.to_string_lossy().to_string())
        .collect();

    // Handle special cases for URL parsing.
    if segments.len() == 1 {
        // Only a single entry, e.g., `/metric/`.
        return (segments[0].to_string(), "".to_string());
    } else if segments.is_empty() {
        // Only the root URL, e.g., `/`.
        return ("/".to_string(), "".to_string());
    }

    // Extract the prefix by joining all but the last segment.
    let prefix = segments
        .iter()
        .take(segments.len() - 1)
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("/");

    // The resource is the last segment in the URL path.
    let resource = segments.last().take().unwrap().to_string();

    (prefix, resource)
}

fn list_keys(tbon: &Tbon, _req: &Request) -> Response {
    match tbon.list_keys() {
        Ok(k) => Response::json(&k),
        Err(e) => Response::text(format!("Error processing list: {}", e)).with_status_code(502),
    }
}

fn do_histogram(tbon: &Tbon, key: &str) -> Response {
    match tbon.histogram(key) {
        Ok((ts, h)) => {
            let hist = TbonHistogram::from_hist(h, ts);
            if hist.buckets.is_empty() {
                return Response::text(format!("No datapoints found for key '{}'", key))
                    .with_status_code(404);
            }
            Response::json(&hist)
        }
        Err(e) => {
            Response::text(format!("Error processing histogram: {}", e)).with_status_code(502)
        }
    }
}

fn do_values(tbon: &Tbon, key: &str) -> Response {
    match tbon.values(key) {
        Ok(hashmap) => {
            if hashmap.is_empty() {
                return Response::text(format!("No data found for key '{}'", key))
                    .with_status_code(404);
            }

            Response::json(&hashmap)
        }
        Err(e) => Response::text(format!("Error processing values: {}", e)).with_status_code(502),
    }
}

static VIEW_PAGE: &str = std::include_str!("../static/view.html");

fn main_page() -> Response {
    Response::html(VIEW_PAGE).with_status_code(200)
}

pub fn serve_clients(tbon: Tbon) -> Result<()> {
    let srv = Server::new("0.0.0.0:1871", move |request| {
        let url = request.url();

        let (prefix, resource) = parse_url(&url);

        match prefix.as_str() {
            "/" => main_page(),
            "keys" => list_keys(&tbon, request),
            "hist" => do_histogram(&tbon, &resource),
            "values" => do_values(&tbon, &resource),
            _ => Response::text("No such API endpoint").with_status_code(404),
        }
    })
    .unwrap();

    srv.run();

    Ok(())
}
