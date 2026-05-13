use chrono::{DateTime, Local};
use imap::{types::Flag, Session};
use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::authentication::Credentials,
    SmtpTransport, Transport,
};
use mailparse::{parse_header, parse_mail, MailHeaderMap};
use native_tls::{TlsConnector, TlsStream};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::TcpStream,
    path::PathBuf,
};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

type ImapSession = Session<TlsStream<TcpStream>>;

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    id: String,
    name: String,
    address: String,
    #[serde(default = "default_true")]
    enabled: bool,
    imap_host: String,
    imap_port: u16,
    smtp_host: String,
    smtp_port: u16,
    username: String,
    password: String,
    use_tls: bool,
    default_sender: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MailStorageSettings {
    mail_root_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MailMessageSummary {
    uid: u32,
    account_id: String,
    account_name: String,
    subject: String,
    from: String,
    to: Vec<String>,
    date: String,
    unread: bool,
    preview: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MailMessageDetail {
    uid: u32,
    account_id: String,
    account_name: String,
    subject: String,
    from: String,
    to: Vec<String>,
    cc: Vec<String>,
    date: String,
    unread: bool,
    text_body: String,
    html_body: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredMailMessage {
    uid: u32,
    account_id: String,
    account_name: String,
    subject: String,
    from: String,
    to: Vec<String>,
    cc: Vec<String>,
    date: String,
    unread: bool,
    preview: String,
    text_body: String,
    html_body: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MailIndexEntry {
    uid: u32,
    account_id: String,
    account_name: String,
    subject: String,
    from: String,
    to: Vec<String>,
    date: String,
    unread: bool,
    preview: String,
    file_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccountInput {
    id: Option<String>,
    name: String,
    address: String,
    enabled: bool,
    imap_host: String,
    imap_port: u16,
    smtp_host: String,
    smtp_port: u16,
    username: String,
    password: String,
    use_tls: bool,
    default_sender: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMailInput {
    account_id: String,
    to: Vec<String>,
    cc: Vec<String>,
    subject: String,
    body: String,
    is_html: bool,
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("鏃犳硶鑾峰彇搴旂敤鏁版嵁鐩綍: {error}"))?;

    fs::create_dir_all(&app_dir).map_err(|error| format!("鏃犳硶鍒涘缓搴旂敤鏁版嵁鐩綍: {error}"))?;
    Ok(app_dir)
}

fn settings_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

fn default_mail_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app_data_dir(app)?.join("mail_cache");
    fs::create_dir_all(&root).map_err(|error| format!("鏃犳硶鍒涘缓榛樿閭欢鐩綍: {error}"))?;
    Ok(root)
}

fn normalize_or_create_dir(path: &std::path::Path) -> Result<PathBuf, String> {
    fs::create_dir_all(path).map_err(|error| format!("鏃犳硶鍒涘缓鐩綍: {error}"))?;
    fs::canonicalize(path).map_err(|error| format!("鏃犳硶瑙ｆ瀽鐩綍璺緞: {error}"))
}

fn read_mail_storage_settings(app: &AppHandle) -> Result<MailStorageSettings, String> {
    let settings_path = settings_file_path(app)?;
    if !settings_path.exists() {
        return Ok(MailStorageSettings {
            mail_root_path: default_mail_root(app)?.to_string_lossy().to_string(),
        });
    }

    let content =
        fs::read_to_string(&settings_path).map_err(|error| format!("鏃犳硶璇诲彇璁剧疆鏂囦欢: {error}"))?;
    let settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("鏃犳硶瑙ｆ瀽璁剧疆鏂囦欢: {error}"))?;

    let mail_root_path = settings
        .get("mailRootPath")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or(display_path(&default_mail_root(app)?));

    Ok(MailStorageSettings { mail_root_path })
}

fn display_path(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        raw.strip_prefix(r"\\?\")
            .map(ToString::to_string)
            .unwrap_or(raw)
    }

    #[cfg(not(windows))]
    {
        raw
    }
}

fn accounts_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("mail_accounts.json"))
}

fn cache_root_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let settings = read_mail_storage_settings(app)?;
    normalize_or_create_dir(std::path::Path::new(&settings.mail_root_path))
}

fn message_store_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = cache_root_dir(app)?.join("messages");
    fs::create_dir_all(&dir).map_err(|error| format!("鏃犳硶鍒涘缓閭欢鏂囦欢鐩綍: {error}"))?;
    Ok(dir)
}

fn index_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(cache_root_dir(app)?.join("mail_index.json"))
}

fn account_message_dir(app: &AppHandle, account_id: &str) -> Result<PathBuf, String> {
    let dir = message_store_dir(app)?.join(account_id);
    fs::create_dir_all(&dir).map_err(|error| format!("鏃犳硶鍒涘缓璐﹀彿閭欢鐩綍: {error}"))?;
    Ok(dir)
}

fn message_file_name(uid: u32) -> String {
    format!("{uid}.json")
}

fn message_file_path(app: &AppHandle, account_id: &str, uid: u32) -> Result<PathBuf, String> {
    Ok(account_message_dir(app, account_id)?.join(message_file_name(uid)))
}

fn mail_key(account_id: &str, uid: u32) -> String {
    format!("{account_id}:{uid}")
}

fn dedupe_mail_index(entries: Vec<MailIndexEntry>) -> Vec<MailIndexEntry> {
    let mut map: HashMap<String, MailIndexEntry> = HashMap::new();

    for entry in entries {
        map.insert(mail_key(&entry.account_id, entry.uid), entry);
    }

    let mut deduped: Vec<MailIndexEntry> = map.into_values().collect();
    deduped.sort_by(|left, right| right.date.cmp(&left.date));
    deduped
}

fn compact_mail_index(entries: Vec<MailIndexEntry>) -> Vec<MailIndexEntry> {
    dedupe_mail_index(entries)
        .into_iter()
        .map(|entry| MailIndexEntry {
            uid: entry.uid,
            account_id: entry.account_id,
            account_name: entry.account_name,
            subject: entry.subject,
            from: entry.from,
            to: entry.to,
            date: entry.date,
            unread: entry.unread,
            preview: entry.preview,
            file_name: entry.file_name,
        })
        .collect()
}

fn read_accounts(app: &AppHandle) -> Result<Vec<MailAccount>, String> {
    let file_path = accounts_file_path(app)?;

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(file_path).map_err(|error| format!("鏃犳硶璇诲彇閭璐﹀彿閰嶇疆: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("鏃犳硶瑙ｆ瀽閭璐﹀彿閰嶇疆: {error}"))
}

fn write_accounts(app: &AppHandle, accounts: &[MailAccount]) -> Result<(), String> {
    let file_path = accounts_file_path(app)?;
    let content = serde_json::to_string_pretty(accounts)
        .map_err(|error| format!("鏃犳硶搴忓垪鍖栭偖绠辫处鍙烽厤缃? {error}"))?;

    fs::write(file_path, content).map_err(|error| format!("鏃犳硶淇濆瓨閭璐﹀彿閰嶇疆: {error}"))
}

fn read_mail_index(app: &AppHandle) -> Result<Vec<MailIndexEntry>, String> {
    let file_path = index_file_path(app)?;

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(file_path).map_err(|error| format!("鏃犳硶璇诲彇閭欢绱㈠紩: {error}"))?;
    serde_json::from_str::<Vec<MailIndexEntry>>(&content)
        .map(compact_mail_index)
        .map_err(|error| format!("鏃犳硶瑙ｆ瀽閭欢绱㈠紩: {error}"))
}

fn write_mail_index(app: &AppHandle, entries: &[MailIndexEntry]) -> Result<(), String> {
    let file_path = index_file_path(app)?;
    let compact_entries = compact_mail_index(entries.to_vec());
    let content = serde_json::to_string_pretty(&compact_entries)
        .map_err(|error| format!("鏃犳硶搴忓垪鍖栭偖浠剁储寮? {error}"))?;

    fs::write(file_path, content).map_err(|error| format!("鏃犳硶淇濆瓨閭欢绱㈠紩: {error}"))
}

fn read_message_file(app: &AppHandle, account_id: &str, uid: u32) -> Result<StoredMailMessage, String> {
    let file_path = message_file_path(app, account_id, uid)?;
    let content =
        fs::read_to_string(file_path).map_err(|error| format!("鏃犳硶璇诲彇閭欢鏂囦欢: {error}"))?;

    serde_json::from_str(&content).map_err(|error| format!("鏃犳硶瑙ｆ瀽閭欢鏂囦欢: {error}"))
}

fn write_message_file(app: &AppHandle, message: &StoredMailMessage) -> Result<String, String> {
    let file_path = message_file_path(app, &message.account_id, message.uid)?;
    let file_name = message_file_name(message.uid);
    let content = serde_json::to_string_pretty(message)
        .map_err(|error| format!("鏃犳硶搴忓垪鍖栭偖浠跺唴瀹? {error}"))?;

    fs::write(file_path, content).map_err(|error| format!("鏃犳硶淇濆瓨閭欢鏂囦欢: {error}"))?;
    Ok(file_name)
}

fn normalize_accounts(mut accounts: Vec<MailAccount>) -> Vec<MailAccount> {
    let default_count = accounts
        .iter()
        .filter(|account| account.enabled && account.default_sender)
        .count();

    if default_count == 0 {
        if let Some(first) = accounts.iter_mut().find(|account| account.enabled) {
            first.default_sender = true;
        }
    } else if default_count > 1 {
        let mut seen_default = false;
        for account in &mut accounts {
            if account.enabled && account.default_sender && !seen_default {
                seen_default = true;
            } else {
                account.default_sender = false;
            }
        }
    }

    for account in &mut accounts {
        if !account.enabled {
            account.default_sender = false;
        }
    }

    accounts
}

fn connect_imap(account: &MailAccount) -> Result<ImapSession, String> {
    let tls = TlsConnector::builder()
        .build()
        .map_err(|error| format!("鏃犳硶鍒涘缓 TLS 杩炴帴: {error}"))?;

    let client = imap::connect(
        (account.imap_host.as_str(), account.imap_port),
        account.imap_host.as_str(),
        &tls,
    )
    .map_err(|error| {
        format!(
            "鏃犳硶杩炴帴 IMAP 鏈嶅姟鍣?{}:{}: {error}",
            account.imap_host, account.imap_port
        )
    })?;

    client
        .login(account.username.as_str(), account.password.as_str())
        .map_err(|(error, _)| format!("閭璐﹀彿 {} 鐧诲綍澶辫触: {error}", account.address))
}

fn decode_mime_value(value: &[u8]) -> String {
    let header_line = format!("X-Temp: {}\r\n", String::from_utf8_lossy(value));
    parse_header(header_line.as_bytes())
        .map(|(header, _)| header.get_value())
        .unwrap_or_else(|_| String::from_utf8_lossy(value).trim().to_string())
}

fn decode_header(value: Option<&[u8]>) -> String {
    value.map(decode_mime_value).unwrap_or_default()
}

fn format_local_date<Tz>(date: DateTime<Tz>) -> String
where
    Tz: chrono::TimeZone,
    Tz::Offset: std::fmt::Display,
{
    date.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn parse_date(date: &str) -> Option<String> {
    let trimmed = date.trim();
    if trimmed.is_empty() {
        return None;
    }

    DateTime::parse_from_rfc2822(trimmed)
        .map(format_local_date)
        .or_else(|_| DateTime::parse_from_rfc3339(trimmed).map(format_local_date))
        .ok()
}

fn format_address(name: Option<&[u8]>, mailbox: Option<&[u8]>, host: Option<&[u8]>) -> String {
    let mailbox = mailbox
        .map(|value| String::from_utf8_lossy(value).to_string())
        .unwrap_or_default();
    let host = host
        .map(|value| String::from_utf8_lossy(value).to_string())
        .unwrap_or_default();
    let name = name
        .map(decode_mime_value)
        .filter(|value| !value.trim().is_empty());

    if mailbox.is_empty() || host.is_empty() {
        return name.unwrap_or_default();
    }

    let email = format!("{mailbox}@{host}");
    if let Some(display_name) = name {
        format!("{display_name} <{email}>")
    } else {
        email
    }
}

fn extract_address_list(addresses: Option<&Vec<imap_proto::types::Address<'_>>>) -> Vec<String> {
    addresses
        .map(|items| {
            items
                .iter()
                .map(|item| format_address(item.name, item.mailbox, item.host))
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn html_to_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut previous_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !previous_space {
                    text.push(' ');
                    previous_space = true;
                }
            }
            _ if in_tag => {}
            '\r' | '\n' | '\t' => {
                if !previous_space {
                    text.push(' ');
                    previous_space = true;
                }
            }
            _ => {
                text.push(ch);
                previous_space = ch.is_whitespace();
            }
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn message_preview(content: &str) -> String {
    content.chars().take(120).collect()
}

fn parsed_text_body(parsed: &mailparse::ParsedMail<'_>) -> (String, Option<String>) {
    fn walk_parts(
        mail: &mailparse::ParsedMail<'_>,
        text: &mut Option<String>,
        html: &mut Option<String>,
    ) {
        if mail.subparts.is_empty() {
            let mime = mail.ctype.mimetype.to_ascii_lowercase();
            if mime == "text/plain" && text.is_none() {
                *text = mail.get_body().ok();
            }
            if mime == "text/html" && html.is_none() {
                *html = mail.get_body().ok();
            }
            return;
        }

        for part in &mail.subparts {
            walk_parts(part, text, html);
        }
    }

    let mut text = None;
    let mut html = None;
    walk_parts(parsed, &mut text, &mut html);

    (
        text.unwrap_or_else(|| {
            html.as_deref()
                .map(html_to_text)
                .unwrap_or_else(|| parsed.get_body().unwrap_or_default())
        }),
        html,
    )
}

fn build_preview(parsed: &mailparse::ParsedMail<'_>) -> String {
    let (text_body, html_body) = parsed_text_body(parsed);
    let source = if text_body.trim().is_empty() {
        html_body
            .as_deref()
            .map(html_to_text)
            .unwrap_or_default()
    } else {
        text_body
    };

    message_preview(&source)
}

fn is_unread(flags: &[Flag<'_>]) -> bool {
    !flags.iter().any(|flag| matches!(flag, Flag::Seen))
}

fn to_summary(entry: &MailIndexEntry) -> MailMessageSummary {
    MailMessageSummary {
        uid: entry.uid,
        account_id: entry.account_id.clone(),
        account_name: entry.account_name.clone(),
        subject: entry.subject.clone(),
        from: entry.from.clone(),
        to: entry.to.clone(),
        date: entry.date.clone(),
        unread: entry.unread,
        preview: entry.preview.clone(),
    }
}

fn to_detail(message: &StoredMailMessage) -> MailMessageDetail {
    MailMessageDetail {
        uid: message.uid,
        account_id: message.account_id.clone(),
        account_name: message.account_name.clone(),
        subject: message.subject.clone(),
        from: message.from.clone(),
        to: message.to.clone(),
        cc: message.cc.clone(),
        date: message.date.clone(),
        unread: message.unread,
        text_body: message.text_body.clone(),
        html_body: message.html_body.clone(),
    }
}

fn to_index_entry(message: &StoredMailMessage, file_name: String) -> MailIndexEntry {
    MailIndexEntry {
        uid: message.uid,
        account_id: message.account_id.clone(),
        account_name: message.account_name.clone(),
        subject: message.subject.clone(),
        from: message.from.clone(),
        to: message.to.clone(),
        date: message.date.clone(),
        unread: message.unread,
        preview: message.preview.clone(),
        file_name,
    }
}

fn sync_account_messages(account: &MailAccount, limit: usize) -> Result<Vec<StoredMailMessage>, String> {
    let mut session = connect_imap(account)?;
    session
        .select("INBOX")
        .map_err(|error| format!("session邮件拉取报错 {} 信息INBOX: {error}", account.address))?;

    let mut uids: Vec<u32> = session
        .uid_search("ALL")
        .map_err(|error| format!("无法读取 {} 的邮件UID列表: {error}", account.address))?
        .into_iter()
        .collect();

    uids.sort_unstable();
    let selected: Vec<u32> = uids.into_iter().rev().take(limit).collect();

    if selected.is_empty() {
        let _ = session.logout();
        return Ok(Vec::new());
    }

    let uid_set = selected
        .iter()
        .map(|uid| uid.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let fetched = session
        .uid_fetch(uid_set, "(UID RFC822 ENVELOPE FLAGS)")
        .map_err(|error| format!("无法拉取 {} 的邮件内容: {error}", account.address))?;

    let mut messages = Vec::new();
    let mut seen_uids = HashSet::new();

    for mail in fetched.iter() {
        let uid = mail.uid.unwrap_or_default();
        if uid == 0 || !seen_uids.insert(uid) {
            continue;
        }

        let Some(envelope) = mail.envelope() else {
            continue;
        };
        let Some(body) = mail.body() else {
            continue;
        };
        let Ok(parsed) = parse_mail(body) else {
            continue;
        };

        let subject = parsed
            .headers
            .get_first_value("Subject")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| decode_header(envelope.subject));
        let date = parsed
            .headers
            .get_first_value("Date")
            .and_then(|value| parse_date(&value))
            .or_else(|| {
                envelope
                    .date
                    .and_then(|value| parse_date(&String::from_utf8_lossy(value)))
            })
            .or_else(|| mail.internal_date().map(format_local_date))
            .unwrap_or_else(|| Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        let preview = build_preview(&parsed);
        let (text_body, html_body) = parsed_text_body(&parsed);

        messages.push(StoredMailMessage {
            uid,
            account_id: account.id.clone(),
            account_name: account.name.clone(),
            subject: if subject.trim().is_empty() {
                "(鏃犱富棰?".into()
            } else {
                subject
            },
            from: extract_address_list(envelope.from.as_ref()).join(", "),
            to: extract_address_list(envelope.to.as_ref()),
            cc: extract_address_list(envelope.cc.as_ref()),
            date,
            unread: is_unread(mail.flags()),
            preview,
            text_body,
            html_body,
        });
    }

    let _ = session.logout();
    Ok(messages)
}

#[tauri::command]
pub fn get_mail_accounts(app: AppHandle) -> Result<Vec<MailAccount>, String> {
    read_accounts(&app)
}

#[tauri::command]
pub fn save_mail_account(
    app: AppHandle,
    account: MailAccountInput,
) -> Result<Vec<MailAccount>, String> {
    let mut accounts = read_accounts(&app)?;
    let account_id = account.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let next = MailAccount {
        id: account_id.clone(),
        name: account.name,
        address: account.address,
        enabled: account.enabled,
        imap_host: account.imap_host,
        imap_port: account.imap_port,
        smtp_host: account.smtp_host,
        smtp_port: account.smtp_port,
        username: account.username,
        password: account.password,
        use_tls: account.use_tls,
        default_sender: account.default_sender,
    };

    if let Some(existing) = accounts.iter_mut().find(|item| item.id == account_id) {
        *existing = next;
    } else {
        accounts.push(next);
    }

    let accounts = normalize_accounts(accounts);
    write_accounts(&app, &accounts)?;
    Ok(accounts)
}

#[tauri::command]
pub fn delete_mail_account(app: AppHandle, account_id: String) -> Result<Vec<MailAccount>, String> {
    let accounts = read_accounts(&app)?;
    let next: Vec<MailAccount> = accounts
        .into_iter()
        .filter(|item| item.id != account_id)
        .collect();
    let next = normalize_accounts(next);
    write_accounts(&app, &next)?;

    let index = dedupe_mail_index(read_mail_index(&app)?);
    let filtered: Vec<MailIndexEntry> = index
        .into_iter()
        .filter(|item| item.account_id != account_id)
        .collect();
    write_mail_index(&app, &filtered)?;

    let account_dir = message_store_dir(&app)?.join(&account_id);
    if account_dir.exists() {
        fs::remove_dir_all(&account_dir).map_err(|error| format!("鏃犳硶鍒犻櫎璐﹀彿閭欢鐩綍: {error}"))?;
    }

    Ok(next)
}

#[tauri::command]
pub fn fetch_mail_messages(
    app: AppHandle,
    _limit_per_account: Option<u32>,
) -> Result<Vec<MailMessageSummary>, String> {
    let mut messages: Vec<MailMessageSummary> = dedupe_mail_index(read_mail_index(&app)?)
        .into_iter()
        .map(|item| to_summary(&item))
        .collect();

    messages.sort_by(|left, right| right.date.cmp(&left.date));
    Ok(messages)
}

#[tauri::command]
pub fn sync_mail_messages(
    app: AppHandle,
    limit_per_account: Option<u32>,
) -> Result<Vec<MailMessageSummary>, String> {
    let accounts = read_accounts(&app)?;
    let limit = limit_per_account.unwrap_or(20).max(1) as usize;
    let existing_index = dedupe_mail_index(read_mail_index(&app)?);
    let mut index_map: HashMap<String, MailIndexEntry> = existing_index
        .into_iter()
        .map(|entry| (mail_key(&entry.account_id, entry.uid), entry))
        .collect();

    for account in accounts.iter().filter(|account| account.enabled) {
        let messages = sync_account_messages(account, limit)?;

        for message in messages {
            let key = mail_key(&message.account_id, message.uid);
            let file_name = match index_map.get(&key) {
                Some(existing) => existing.file_name.clone(),
                None => write_message_file(&app, &message)?,
            };

            if index_map.contains_key(&key) {
                let _ = write_message_file(&app, &message)?;
            }

            index_map.insert(key, to_index_entry(&message, file_name));
        }
    }

    let mut index_entries: Vec<MailIndexEntry> = index_map.into_values().collect();
    index_entries.sort_by(|left, right| right.date.cmp(&left.date));
    write_mail_index(&app, &index_entries)?;

    Ok(index_entries.iter().map(to_summary).collect())
}

#[tauri::command]
pub fn fetch_mail_detail(app: AppHandle, account_id: String, uid: u32) -> Result<MailMessageDetail, String> {
    let mut index = dedupe_mail_index(read_mail_index(&app)?);
    let index_entry = index
        .iter_mut()
        .find(|item| item.account_id == account_id && item.uid == uid)
        .ok_or_else(|| "Mail not found in local cache. Please sync messages first.".to_string())?;

    let mut message = read_message_file(&app, &account_id, uid)?;
    message.unread = false;
    index_entry.unread = false;

    write_message_file(&app, &message)?;
    write_mail_index(&app, &index)?;

    Ok(to_detail(&message))
}

#[tauri::command]
pub fn send_mail(app: AppHandle, input: SendMailInput) -> Result<(), String> {
    let accounts = read_accounts(&app)?;
    let account = accounts
        .into_iter()
        .find(|item| item.id == input.account_id && item.enabled)
        .ok_or_else(|| "Failed to find the enabled mail account".to_string())?;

    let from_mailbox: Mailbox = format!("{} <{}>", account.name, account.address)
        .parse()
        .map_err(|error| format!("Invalid sender address: {error}"))?;

    let mut builder = Message::builder().from(from_mailbox).subject(input.subject);

    for recipient in input.to {
        builder = builder.to(
            recipient
                .parse()
                .map_err(|error| format!("Invalid recipient address: {error}"))?,
        );
    }

    for recipient in input.cc {
        builder = builder.cc(
            recipient
                .parse()
                .map_err(|error| format!("Invalid cc address: {error}"))?,
        );
    }

    let message = if input.is_html {
        builder
            .header(ContentType::TEXT_HTML)
            .body(input.body)
            .map_err(|error| format!("Failed to build message body: {error}"))?
    } else {
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(input.body)
            .map_err(|error| format!("Failed to build message body: {error}"))?
    };

    let credentials = Credentials::new(account.username, account.password);
    let transport = if account.use_tls {
        SmtpTransport::relay(&account.smtp_host)
            .map_err(|error| format!("Failed to create SMTP TLS transport: {error}"))?
            .port(account.smtp_port)
            .credentials(credentials)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&account.smtp_host)
            .port(account.smtp_port)
            .credentials(credentials)
            .build()
    };

    transport
        .send(&message)
        .map_err(|error| format!("Failed to send mail: {error}"))?;

    Ok(())
}
