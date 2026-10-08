//! Emails the admin sends to players: reusable templates with placeholders, a branded HTML layout generated from plain text,
//! audiences (everyone, a group, chosen players), opt-out links, and a log of what went out.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::embeds::{fill, plain, Vars};
use crate::error::{AppError, AppResult};
use crate::routes::connections;
use crate::state::AppState;
use crate::store;
use axum::extract::{Path, Query};
use axum::response::Html;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

const MAX_RECIPIENTS: usize = 2000;

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(default)]
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub body: String,
}

fn defaults() -> Vec<EmailTemplate> {
    vec![
        EmailTemplate {
            id: "welcome".into(),
            name: "Welcome".into(),
            subject: "Welcome to {brand}, {player}!".into(),
            body: "# Welcome, {player}!\n\nYour account on **{brand}** is ready. Download the launcher, sign in and join the server.\n\n[button: Open {brand}]({panel_url})\n\nSee you in game!".into(),
        },
        EmailTemplate {
            id: "news".into(),
            name: "News & updates".into(),
            subject: "News from {brand}".into(),
            body: "Hi {player},\n\nHere is what is new on {brand}:\n\n- First thing\n- Second thing\n\n[button: Read more]({panel_url})".into(),
        },
        EmailTemplate {
            id: "event".into(),
            name: "Event announcement".into(),
            subject: "{brand} event: don't miss it".into(),
            body: "# A new event is starting\n\nHi {player}, you are level **{level}**{rank_title_suffix}. Come and join in!\n\n[button: Join now]({panel_url})".into(),
        },
    ]
}

pub async fn templates(state: &AppState) -> AppResult<Vec<EmailTemplate>> {
    let stored: Vec<EmailTemplate> = store::kv_get(state, "email_templates").await?;
    Ok(if stored.is_empty() { defaults() } else { stored })
}

pub const PLACEHOLDERS: [&str; 11] =
    ["player", "uuid", "email", "level", "rank_title", "rank_title_suffix", "guild", "guild_tag", "brand", "panel_url", "unsubscribe_url"];

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn safe_url(u: &str) -> Option<String> {
    let u = u.trim();
    (u.starts_with("https://") || u.starts_with("http://")).then(|| esc(u))
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
        let line = esc(raw.trim_end());
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
        brand = esc(brand),
    )
}

fn unsub_secret_key() -> &'static str {
    "email_unsubscribe_secret"
}

async fn unsub_secret(state: &AppState) -> AppResult<String> {
    let mut s: String = store::kv_get(state, unsub_secret_key()).await?;
    if s.is_empty() {
        s = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        store::kv_set(state, unsub_secret_key(), &s).await?;
    }
    Ok(s)
}

async fn unsub_token(state: &AppState, uuid: &str) -> AppResult<String> {
    let secret = unsub_secret(state).await?;
    let digest = Sha256::digest(format!("{secret}:{uuid}").as_bytes());
    Ok(digest.iter().take(16).map(|b| format!("{b:02x}")).collect())
}

struct Person {
    uuid: String,
    username: String,
    email: String,
}

async fn vars_for(state: &AppState, p: &Person) -> AppResult<Vars> {
    let brand = store::branding(state).await?.name;
    let panel = state.cfg.public_url.clone().unwrap_or_default().trim_end_matches('/').to_string();
    let mut v = Vars::new();
    let level: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT global_level, title FROM user_levels WHERE uuid = ?").bind(&p.uuid).fetch_optional(&state.db).await?;
    let (lvl, title) = level.unwrap_or((1, None));
    let title = plain(&title.unwrap_or_default());
    let guild: Option<(String, String)> =
        sqlx::query_as("SELECT g.name, g.tag FROM guild_members m JOIN guilds g ON g.id = m.guild_id WHERE m.uuid = ?")
            .bind(&p.uuid)
            .fetch_optional(&state.db)
            .await?;
    let (gn, gt) = guild.unwrap_or_default();
    for (k, val) in [
        ("player", plain(&p.username)),
        ("uuid", p.uuid.clone()),
        ("email", p.email.clone()),
        ("level", lvl.to_string()),
        ("rank_title_suffix", if title.is_empty() { String::new() } else { format!(" ({title})") }),
        ("rank_title", title),
        ("guild", plain(&gn)),
        ("guild_tag", plain(&gt)),
        ("brand", brand),
        ("panel_url", panel.clone()),
        ("unsubscribe_url", format!("{panel}/api/v1/email/unsubscribe?u={}&t={}", p.uuid, unsub_token(state, &p.uuid).await?)),
    ] {
        v.insert(k.into(), val);
    }
    Ok(v)
}

struct Rendered {
    subject: String,
    text: String,
    html: String,
}

async fn build(state: &AppState, subject: &str, body: &str, important: bool, p: &Person) -> AppResult<Rendered> {
    let vars = vars_for(state, p).await?;
    let subject = fill(subject, &vars).replace(['\r', '\n'], " ");
    let text = fill(body, &vars);
    let brand = vars["brand"].clone();
    let accent = store::branding(state).await?.colors.accent;
    let footer_text = if important {
        format!("This message is about your {brand} account.")
    } else {
        format!(
            "You are receiving this because you have an account on {brand}. <a href=\"{}\" style=\"color:#8d97b8\">Unsubscribe</a>",
            esc(&vars["unsubscribe_url"])
        )
    };
    let mut plain_text = text.clone();
    if !important {
        plain_text.push_str(&format!("\n\n--\nUnsubscribe: {}", vars["unsubscribe_url"]));
    }
    Ok(Rendered { subject, text: plain_text, html: html(&text, &brand, &accent, &footer_text) })
}

// ---------------------------------------------------------------------------
// Audiences
// ---------------------------------------------------------------------------

#[derive(Deserialize, Clone, Default)]
#[serde(default)]
pub struct Audience {
    /// `all`, `group` or `players`
    kind: String,
    group_id: i64,
    /// Player names, for `players`
    names: Vec<String>,
}

async fn recipients(state: &AppState, a: &Audience, important: bool) -> AppResult<Vec<Person>> {
    let optout = if important { "" } else { " AND u.email_optout = 0" };
    let base = format!(
        "SELECT u.uuid, u.username, u.email FROM users u WHERE u.status = 'active' AND u.email IS NOT NULL AND u.email <> ''{optout}"
    );
    let rows: Vec<(String, String, String)> = match a.kind.as_str() {
        "all" => sqlx::query_as(&format!("{base} ORDER BY u.id")).fetch_all(&state.db).await?,
        "group" => {
            sqlx::query_as(&format!("{base} AND u.id IN (SELECT user_id FROM user_groups WHERE group_id = ?) ORDER BY u.id"))
                .bind(a.group_id)
                .fetch_all(&state.db)
                .await?
        }
        "players" => {
            let mut out = Vec::new();
            for n in a.names.iter().take(500) {
                let r: Option<(String, String, String)> =
                    sqlx::query_as(&format!("{base} AND u.username = ? COLLATE NOCASE")).bind(n.trim()).fetch_optional(&state.db).await?;
                out.extend(r);
            }
            out
        }
        _ => return Err(AppError::bad_request("choose who should get the email")),
    };
    Ok(rows.into_iter().take(MAX_RECIPIENTS).map(|(uuid, username, email)| Person { uuid, username, email }).collect())
}

// ---------------------------------------------------------------------------
// Admin endpoints
// ---------------------------------------------------------------------------

pub async fn get_all(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let conn = connections::settings(&state).await?;
    let log: Vec<(i64, String, String, i64, i64, i64, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT id, subject, audience, total, sent, failed, last_error, started_at, finished_at FROM email_log ORDER BY id DESC LIMIT 20",
    )
    .fetch_all(&state.db)
    .await?;
    let groups: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM groups ORDER BY name").fetch_all(&state.db).await?;
    Ok(Json(json!({
        "configured": !conn.resend_api_key.is_empty() && !conn.sender_email.is_empty(),
        "templates": templates(&state).await?,
        "placeholders": PLACEHOLDERS,
        "groups": groups.into_iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "log": log.into_iter().map(|(id, subject, audience, total, sent, failed, error, started, finished)| json!({
            "id": id, "subject": subject, "audience": audience, "total": total, "sent": sent, "failed": failed, "last_error": error, "started_at": started, "finished_at": finished })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct TemplatesBody {
    templates: Vec<EmailTemplate>,
}

pub async fn put_templates(_: AdminUser, State(state): State<AppState>, Json(b): Json<TemplatesBody>) -> AppResult<Json<Value>> {
    if b.templates.len() > 50 {
        return Err(AppError::bad_request("at most 50 email templates"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for mut t in b.templates {
        t.id = t.id.trim().to_string();
        if t.id.is_empty() || !t.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') || !seen.insert(t.id.clone()) {
            return Err(AppError::bad_request("each template needs its own short id (letters, numbers, - and _)"));
        }
        if t.subject.chars().count() > 200 || t.body.chars().count() > 20_000 || t.name.chars().count() > 80 {
            return Err(AppError::bad_request("a template is too long"));
        }
        out.push(t);
    }
    store::kv_set(&state, "email_templates", &out).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct PreviewBody {
    subject: String,
    body: String,
    #[serde(default)]
    important: bool,
}

async fn sample_person(state: &AppState, admin: &crate::auth::UserRow) -> Person {
    let _ = state;
    Person { uuid: admin.uuid.clone(), username: admin.username.clone(), email: admin.email.clone().unwrap_or_default() }
}

pub async fn preview(admin: AdminUser, State(state): State<AppState>, Json(b): Json<PreviewBody>) -> AppResult<Json<Value>> {
    let who = sample_person(&state, &admin.0).await;
    let r = build(&state, &b.subject, &b.body, b.important, &who).await?;
    Ok(Json(json!({ "subject": r.subject, "text": r.text, "html": r.html })))
}

#[derive(Deserialize)]
pub struct AudienceBody {
    audience: Audience,
    #[serde(default)]
    important: bool,
}

pub async fn audience(_: AdminUser, State(state): State<AppState>, Json(b): Json<AudienceBody>) -> AppResult<Json<Value>> {
    let list = recipients(&state, &b.audience, b.important).await?;
    Ok(Json(
        json!({ "count": list.len(), "sample": list.iter().take(5).map(|p| p.username.clone()).collect::<Vec<_>>(), "limit": MAX_RECIPIENTS }),
    ))
}

#[derive(Deserialize)]
pub struct SendBody {
    subject: String,
    body: String,
    #[serde(default)]
    important: bool,
    #[serde(default)]
    audience: Audience,
    /// Send one test copy to this address instead of to players.
    #[serde(default)]
    test_to: String,
}

pub async fn send(admin: AdminUser, State(state): State<AppState>, Json(b): Json<SendBody>) -> AppResult<Json<Value>> {
    if b.subject.trim().is_empty() || b.body.trim().is_empty() {
        return Err(AppError::bad_request("write a subject and a message first"));
    }
    if !b.test_to.trim().is_empty() {
        let me = sample_person(&state, &admin.0).await;
        let r = build(&state, &format!("[Test] {}", b.subject), &b.body, b.important, &me).await?;
        connections::send_mail(&state, b.test_to.trim(), &r.subject, &r.text, Some(&r.html)).await?;
        return Ok(Json(json!({ "ok": true, "test": true })));
    }
    let people = recipients(&state, &b.audience, b.important).await?;
    if people.is_empty() {
        return Err(AppError::bad_request("nobody to send to: no matching players have an email address (or they opted out)"));
    }
    let label = match b.audience.kind.as_str() {
        "all" => "Everyone".to_string(),
        "group" => sqlx::query_scalar::<_, String>("SELECT name FROM groups WHERE id = ?")
            .bind(b.audience.group_id)
            .fetch_optional(&state.db)
            .await?
            .map(|n| format!("Group: {n}"))
            .unwrap_or_else(|| "A group".into()),
        _ => format!("{} chosen players", people.len()),
    };
    let log_id: i64 = sqlx::query_scalar("INSERT INTO email_log (subject, audience, total, started_at) VALUES (?, ?, ?, ?) RETURNING id")
        .bind(b.subject.chars().take(200).collect::<String>())
        .bind(label)
        .bind(people.len() as i64)
        .bind(crate::db::now())
        .fetch_one(&state.db)
        .await?;
    let total = people.len();
    let bg = state.clone();
    tokio::spawn(async move {
        for p in people {
            let outcome = match build(&bg, &b.subject, &b.body, b.important, &p).await {
                Ok(r) => connections::send_mail(&bg, &p.email, &r.subject, &r.text, Some(&r.html)).await,
                Err(e) => Err(e),
            };
            match outcome {
                Ok(()) => {
                    let _ = sqlx::query("UPDATE email_log SET sent = sent + 1 WHERE id = ?").bind(log_id).execute(&bg.db).await;
                }
                Err(e) => {
                    let _ = sqlx::query("UPDATE email_log SET failed = failed + 1, last_error = ? WHERE id = ?")
                        .bind(e.message.chars().take(200).collect::<String>())
                        .bind(log_id)
                        .execute(&bg.db)
                        .await;
                }
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        let _ = sqlx::query("UPDATE email_log SET finished_at = ? WHERE id = ?").bind(crate::db::now()).bind(log_id).execute(&bg.db).await;
    });
    Ok(Json(json!({ "ok": true, "queued": total, "log_id": log_id })))
}

// ---------------------------------------------------------------------------
// Public: unsubscribe
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UnsubQuery {
    u: String,
    t: String,
}

pub async fn unsubscribe(State(state): State<AppState>, Query(q): Query<UnsubQuery>) -> Html<String> {
    let page = |msg: &str| {
        Html(format!("<!doctype html><meta name=viewport content='width=device-width'><body style=\"font-family:system-ui;background:#0f1220;color:#e9ecff;display:grid;place-items:center;min-height:100vh;margin:0\"><div style=\"max-width:420px;padding:32px;text-align:center\"><h2>{}</h2></div></body>", esc(msg)))
    };
    let ok = matches!(unsub_token(&state, &q.u).await, Ok(t) if t == q.t);
    if !ok {
        return page("That link isn't valid.");
    }
    let _ = sqlx::query("UPDATE users SET email_optout = 1 WHERE uuid = ?").bind(&q.u).execute(&state.db).await;
    page("You are unsubscribed. You will still get emails about your account.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_becomes_safe_branded_html() {
        let h = html("# Hi <b>\n\nHello **world** [site](https://x.example/a?b=1&c=2) [button: Go](https://x.example)\n\n- one\n- two\n\n[bad](javascript:alert(1))", "My <Server>", "#112233", "foot");
        assert!(h.contains("<h1") && h.contains("Hi &lt;b&gt;"));
        assert!(h.contains("<strong>world</strong>"));
        assert!(h.contains("href=\"https://x.example/a?b=1&amp;c=2\""));
        assert!(h.contains("background:#112233;color:#ffffff"), "buttons use the brand colour");
        assert!(h.contains("<li style=\"margin:4px 0\">one</li>"));
        assert!(!h.contains("javascript:"), "only web links survive");
        assert!(h.contains("My &lt;Server&gt;"));
    }
}

// Email template management endpoints
fn check_template_size(name: &str, subject: &str, body: &str) -> AppResult<()> {
    if name.chars().count() > 80 || subject.chars().count() > 200 || body.chars().count() > 20_000 {
        return Err(AppError::bad_request("a template is too long"));
    }
    Ok(())
}

pub async fn list_templates(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let templates = templates(&state).await?;
    Ok(Json(json!({ "templates": templates })))
}

#[derive(Deserialize)]
pub struct CreateTemplateBody {
    id: String,
    name: String,
    subject: String,
    body: String,
}

pub async fn create_template(_: AdminUser, State(state): State<AppState>, Json(body): Json<CreateTemplateBody>) -> AppResult<Json<Value>> {
    let id = body.id.trim();
    if id.is_empty() || id.len() > 50 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::bad_request("Template ID must be 1-50 alphanumeric characters, underscore or dash"));
    }
    
    let mut templates = templates(&state).await?;
    
    if templates.iter().any(|t| t.id == id) {
        return Err(AppError::conflict("Template ID already exists"));
    }
    
    check_template_size(&body.name, &body.subject, &body.body)?;
    let created = EmailTemplate { id: id.to_string(), name: body.name, subject: body.subject, body: body.body };
    templates.push(created.clone());

    store::kv_set(&state, "email_templates", &templates).await?;

    Ok(Json(json!(created)))
}

pub async fn get_template(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let templates = templates(&state).await?;
    
    match templates.iter().find(|t| t.id == id) {
        Some(template) => Ok(Json(json!({ "template": template }))),
        None => Err(AppError::not_found("Template not found"))
    }
}

#[derive(Deserialize)]
pub struct UpdateTemplateBody {
    name: Option<String>,
    subject: Option<String>,
    body: Option<String>,
}

pub async fn update_template(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>, Json(body): Json<UpdateTemplateBody>) -> AppResult<Json<Value>> {
    let mut templates = templates(&state).await?;
    
    let template = templates.iter_mut().find(|t| t.id == id)
        .ok_or_else(|| AppError::not_found("Template not found"))?;
    
    if let Some(name) = body.name {
        template.name = name;
    }
    if let Some(subject) = body.subject {
        template.subject = subject;
    }
    if let Some(body_text) = body.body {
        template.body = body_text;
    }
    check_template_size(&template.name, &template.subject, &template.body)?;
    
    store::kv_set(&state, "email_templates", &templates).await?;
    
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_template(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let mut templates = templates(&state).await?;
    
    let original_len = templates.len();
    templates.retain(|t| t.id != id);
    
    if templates.len() == original_len {
        return Err(AppError::not_found("Template not found"));
    }
    
    store::kv_set(&state, "email_templates", &templates).await?;
    
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct SendTestEmailBody {
    template_id: String,
    recipient: String,
}

pub async fn send_test_email(_: AdminUser, State(state): State<AppState>, Json(body): Json<SendTestEmailBody>) -> AppResult<Json<Value>> {
    let templates = templates(&state).await?;
    
    let template = templates.iter().find(|t| t.id == body.template_id)
        .ok_or_else(|| AppError::not_found("Template not found"))?;
    
    let recipient = body.recipient.trim();
    if recipient.is_empty() || !recipient.contains('@') {
        return Err(AppError::bad_request("Invalid email address"));
    }
    
    let person = Person { uuid: "00000000-0000-0000-0000-000000000000".to_string(), username: "TestPlayer".to_string(), email: recipient.to_string() };
    let rendered = build(&state, &format!("[Test] {}", template.subject), &template.body, true, &person).await?;
    connections::send_mail(&state, recipient, &rendered.subject, &rendered.text, Some(&rendered.html)).await?;

    Ok(Json(json!({ "ok": true, "preview": { "subject": rendered.subject, "html": rendered.html } })))
}
