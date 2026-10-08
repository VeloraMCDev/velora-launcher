//! Generic mail layout; callers supply resolved text and a trusted HTML footer.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn safe_url(u: &str) -> Option<String> {
    let u = u.trim();
    (u.starts_with("https://") || u.starts_with("http://")).then(|| escape(u))
}

/// `**bold**`, `*italic*`, `[text](url)` and `[button: text](url)` inside one already-escaped line.
fn inline(line: &str, accent: &str) -> String {
    let mut out = String::new();
    let mut rest = line.to_string();
    // links and buttons first, from the raw (escaped) text
    while let Some(open) = rest.find('[') {
        let Some(mid) = rest[open..].find("](").map(|i| open + i) else { break };
        let Some(close) = rest[mid..].find(')').map(|i| mid + i) else { break };
        let label = &rest[open + 1..mid];
        let url = rest[mid + 2..close].replace("&amp;", "&");
        out.push_str(&emphasis(&rest[..open]));
        match (safe_url(&url), label.strip_prefix("button:")) {
            (Some(u), Some(text)) => out.push_str(&format!(
                "<a href=\"{u}\" style=\"display:inline-block;padding:12px 24px;border-radius:10px;background:{accent};color:#ffffff;font-weight:700;text-decoration:none\">{}</a>",
                text.trim()
            )),
            (Some(u), None) => out.push_str(&format!("<a href=\"{u}\" style=\"color:{accent}\">{label}</a>")),
            _ => out.push_str(label),
        }
        rest = rest[close + 1..].to_string();
    }
    out.push_str(&emphasis(&rest));
    out
}

fn emphasis(s: &str) -> String {
    let mut out = String::new();
    let mut bold = false;
    for (i, part) in s.split("**").enumerate() {
        if i > 0 {
            out.push_str(if bold { "</strong>" } else { "<strong>" });
            bold = !bold;
        }
        out.push_str(part);
    }
    if bold {
        out.push_str("</strong>");
    }
    out
}

/// Plain text to the branded HTML layout. Everything the admin or a player typed is escaped first.
pub fn html(body: &str, brand: &str, accent: &str, footer: &str) -> String {
    let accent =
        if accent.len() == 7 && accent.starts_with('#') && accent[1..].chars().all(|c| c.is_ascii_hexdigit()) { accent } else { "#8b6cff" };
    let mut content = String::new();
    let mut list = false;
    for raw in body.lines() {
        let line = escape(raw.trim_end());
        let bullet = line.trim_start().strip_prefix("- ").map(str::to_string);
        if let Some(item) = &bullet {
            if !list {
                content.push_str("<ul style=\"margin:0 0 16px;padding-left:22px\">");
                list = true;
            }
            content.push_str(&format!("<li style=\"margin:4px 0\">{}</li>", inline(item, accent)));
            continue;
        }
        if list {
            content.push_str("</ul>");
            list = false;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(h) = line.strip_prefix("# ") {
            content.push_str(&format!("<h1 style=\"margin:0 0 14px;font-size:24px;line-height:1.25\">{}</h1>", inline(h, accent)));
        } else if let Some(h) = line.strip_prefix("## ") {
            content.push_str(&format!("<h2 style=\"margin:18px 0 10px;font-size:18px\">{}</h2>", inline(h, accent)));
        } else {
            content.push_str(&format!("<p style=\"margin:0 0 16px;line-height:1.6\">{}</p>", inline(&line, accent)));
        }
    }
    if list {
        content.push_str("</ul>");
    }
    format!(
        "<!doctype html><html><body style=\"margin:0;padding:0;background:#0f1220\"><table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"background:#0f1220;padding:28px 12px\"><tr><td align=\"center\">\
         <table role=\"presentation\" width=\"560\" cellpadding=\"0\" cellspacing=\"0\" style=\"max-width:560px;width:100%;background:#171a2b;border-radius:16px;overflow:hidden;font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#e9ecff\">\
         <tr><td style=\"height:6px;background:{accent}\"></td></tr>\
         <tr><td style=\"padding:26px 30px 6px;font-size:13px;font-weight:700;letter-spacing:.08em;text-transform:uppercase;color:{accent}\">{brand}</td></tr>\
         <tr><td style=\"padding:6px 30px 26px;font-size:15px\">{content}</td></tr>\
         <tr><td style=\"padding:18px 30px 24px;border-top:1px solid #262a40;font-size:12px;color:#8d97b8\">{footer}</td></tr>\
         </table></td></tr></table></body></html>",
        brand = escape(brand),
    )
}
