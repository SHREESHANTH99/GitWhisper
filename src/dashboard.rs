use crate::error::{AppError, AppResult};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn serve_dashboard(host: &str, port: u16) {
    match serve_dashboard_inner(host, port) {
        Ok(()) => {}
        Err(error) => eprintln!("{error}"),
    }
}

fn serve_dashboard_inner(host: &str, port: u16) -> AppResult<()> {
    let address = format!("{host}:{port}");
    let listener = TcpListener::bind(&address)
        .map_err(|error| AppError::message(format!("Failed to bind dashboard on {address}: {error}")))?;

    println!("Gitwhisper dashboard running at http://{address}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = handle_connection(&mut stream);
            }
            Err(error) => eprintln!("Dashboard connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(stream: &mut TcpStream) -> AppResult<()> {
    let mut buffer = [0; 4096];
    let read = stream.read(&mut buffer)?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();
    let path = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    match path {
        "/" => respond_html(stream, &render_dashboard_page())?,
        "/snapshot.json" => {
            let snapshot = crate::metrics::collect_snapshot()?;
            let body = serde_json::to_string_pretty(&snapshot)?;
            respond_json(stream, &body)?;
        }
        "/snapshot.csv" => {
            let snapshot = crate::metrics::collect_snapshot()?;
            let body = crate::metrics::exporter::snapshot_to_csv(&snapshot);
            respond_csv(stream, &body)?;
        }
        "/healthz" => respond_text(stream, "ok")?,
        _ => respond_not_found(stream)?,
    }

    Ok(())
}

fn respond_html(stream: &mut TcpStream, body: &str) -> AppResult<()> {
    respond(stream, "200 OK", "text/html; charset=utf-8", body)
}

fn respond_json(stream: &mut TcpStream, body: &str) -> AppResult<()> {
    respond(stream, "200 OK", "application/json; charset=utf-8", body)
}

fn respond_csv(stream: &mut TcpStream, body: &str) -> AppResult<()> {
    respond(stream, "200 OK", "text/csv; charset=utf-8", body)
}

fn respond_text(stream: &mut TcpStream, body: &str) -> AppResult<()> {
    respond(stream, "200 OK", "text/plain; charset=utf-8", body)
}

fn respond_not_found(stream: &mut TcpStream) -> AppResult<()> {
    respond(stream, "404 Not Found", "text/plain; charset=utf-8", "Not found")
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) -> AppResult<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn render_dashboard_page() -> String {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Gitwhisper Team Dashboard</title>
  <style>
    :root {
      --bg: #0b1020;
      --panel: rgba(16, 24, 48, 0.78);
      --panel-strong: rgba(20, 31, 61, 0.92);
      --text: #f6f4ec;
      --muted: #b3bfd8;
      --accent: #ff7a18;
      --accent-2: #ffb347;
      --line: rgba(255,255,255,0.08);
      --good: #79d98c;
      --warn: #ffd166;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: Georgia, "Times New Roman", serif;
      color: var(--text);
      background:
        radial-gradient(circle at top left, rgba(255,122,24,0.22), transparent 34%),
        radial-gradient(circle at top right, rgba(255,179,71,0.15), transparent 26%),
        linear-gradient(180deg, #11192f 0%, #0b1020 100%);
      min-height: 100vh;
    }
    .shell {
      width: min(1200px, calc(100% - 32px));
      margin: 24px auto 40px;
    }
    .hero {
      padding: 28px;
      border: 1px solid var(--line);
      border-radius: 24px;
      background: linear-gradient(135deg, rgba(26,38,71,0.94), rgba(12,17,33,0.85));
      box-shadow: 0 24px 80px rgba(0,0,0,0.35);
    }
    h1 { margin: 0; font-size: clamp(2rem, 5vw, 3.6rem); line-height: 0.95; }
    .lede { margin-top: 10px; color: var(--muted); max-width: 70ch; }
    .meta { margin-top: 14px; color: var(--accent-2); font-size: 0.95rem; }
    .grid {
      display: grid;
      gap: 16px;
      margin-top: 18px;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }
    .card {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 18px;
      backdrop-filter: blur(10px);
    }
    .card h2, .card h3 { margin: 0 0 10px; font-size: 1.05rem; letter-spacing: 0.03em; }
    .big { font-size: 2rem; color: var(--accent-2); }
    .section {
      margin-top: 18px;
      padding: 18px;
      border-radius: 22px;
      background: var(--panel-strong);
      border: 1px solid var(--line);
    }
    table { width: 100%; border-collapse: collapse; font-size: 0.96rem; }
    th, td { padding: 10px 8px; border-bottom: 1px solid var(--line); text-align: left; vertical-align: top; }
    th { color: var(--accent-2); font-weight: 600; }
    .pill {
      display: inline-block;
      padding: 4px 10px;
      border-radius: 999px;
      background: rgba(255,255,255,0.07);
      color: var(--muted);
      font-size: 0.82rem;
      margin-right: 6px;
      margin-bottom: 6px;
    }
    .bar {
      height: 10px;
      background: rgba(255,255,255,0.07);
      border-radius: 999px;
      overflow: hidden;
      margin-top: 6px;
    }
    .bar > span {
      display: block;
      height: 100%;
      background: linear-gradient(90deg, var(--accent), var(--accent-2));
    }
    .risk { color: var(--warn); }
    a { color: var(--accent-2); text-decoration: none; }
    @media (max-width: 700px) {
      .shell { width: calc(100% - 20px); }
      .hero { padding: 20px; border-radius: 18px; }
      .section, .card { border-radius: 16px; }
      table, thead, tbody, th, td, tr { display: block; }
      th { display: none; }
      td { padding: 8px 0; }
    }
  </style>
</head>
<body>
  <div class="shell">
    <section class="hero">
      <h1>Gitwhisper Team Dashboard</h1>
      <p class="lede">Live project intelligence for commit activity, ownership concentration, hot files, and recent delivery patterns.</p>
      <div class="meta" id="meta">Loading snapshot...</div>
      <div class="grid" id="overview"></div>
    </section>

    <section class="section">
      <h2>People</h2>
      <div id="people"></div>
    </section>

    <section class="section">
      <h2>Files</h2>
      <div id="files"></div>
    </section>

    <section class="section">
      <h2>Weekly Trend</h2>
      <div id="weeks"></div>
    </section>

    <section class="section">
      <h2>Risks</h2>
      <div id="risks"></div>
    </section>

    <section class="section">
      <h2>Recent Commits</h2>
      <div id="recent"></div>
      <p><a href="/snapshot.json">Download JSON snapshot</a> | <a href="/snapshot.csv">Download CSV snapshot</a></p>
    </section>
  </div>
  <script>
    async function load() {
      const response = await fetch('/snapshot.json', { cache: 'no-store' });
      const data = await response.json();
      document.getElementById('meta').textContent = `Updated ${data.generated_at}`;

      const overview = [
        ['Commits', data.overview.total_commits],
        ['Authors', data.overview.unique_authors],
        ['Files', data.overview.files_touched],
        ['Last 7d', data.overview.commits_last_7d],
      ];
      document.getElementById('overview').innerHTML = overview.map(([label, value]) => `
        <div class="card">
          <h3>${label}</h3>
          <div class="big">${value}</div>
        </div>
      `).join('');

      document.getElementById('people').innerHTML = renderPeople(data.people);
      document.getElementById('files').innerHTML = renderFiles(data.files);
      document.getElementById('weeks').innerHTML = renderWeeks(data.weekly_activity);
      document.getElementById('risks').innerHTML = renderRisks(data.risks);
      document.getElementById('recent').innerHTML = renderRecent(data.recent_commits);
    }

    function escapeHtml(value) {
      return String(value)
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;');
    }

    function renderPeople(people) {
      if (!people.length) return '<p>No commit context has been captured yet.</p>';
      return `
        <table>
          <thead><tr><th>Author</th><th>Commits</th><th>Files</th><th>Top Areas</th></tr></thead>
          <tbody>
            ${people.slice(0, 12).map(person => `
              <tr>
                <td>${escapeHtml(person.author)}</td>
                <td>${person.commits}</td>
                <td>${person.files_touched}</td>
                <td>${person.top_files.map(file => `<span class="pill">${escapeHtml(file)}</span>`).join('')}</td>
              </tr>
            `).join('')}
          </tbody>
        </table>`;
    }

    function renderFiles(files) {
      if (!files.length) return '<p>No file activity has been captured yet.</p>';
      return `
        <table>
          <thead><tr><th>File</th><th>Commits</th><th>Top Owner</th><th>Ownership</th></tr></thead>
          <tbody>
            ${files.slice(0, 15).map(file => `
              <tr>
                <td>${escapeHtml(file.path)}</td>
                <td>${file.commits}</td>
                <td>${escapeHtml(file.top_author)}</td>
                <td>
                  ${(file.top_author_share * 100).toFixed(0)}%
                  <div class="bar"><span style="width:${Math.max(file.top_author_share * 100, 4)}%"></span></div>
                </td>
              </tr>
            `).join('')}
          </tbody>
        </table>`;
    }

    function renderWeeks(weeks) {
      if (!weeks.length) return '<p>No weekly activity available yet.</p>';
      const max = Math.max(...weeks.map(item => item.commits), 1);
      return weeks.slice(-12).map(item => `
        <div class="card" style="margin-bottom:10px">
          <div>${escapeHtml(item.week)}: ${item.commits} commits</div>
          <div class="bar"><span style="width:${(item.commits / max) * 100}%"></span></div>
        </div>
      `).join('');
    }

    function renderRisks(risks) {
      if (!risks.length) return '<p>No collaboration risks detected in the current snapshot.</p>';
      return risks.slice(0, 12).map(risk => `
        <div class="card" style="margin-bottom:10px">
          <h3 class="risk">${escapeHtml(risk.kind)}</h3>
          <div>${escapeHtml(risk.subject)}</div>
          <p>${escapeHtml(risk.detail)}</p>
        </div>
      `).join('');
    }

    function renderRecent(commits) {
      if (!commits.length) return '<p>No recent commits found.</p>';
      return `
        <table>
          <thead><tr><th>Commit</th><th>Author</th><th>Subject</th><th>Files</th></tr></thead>
          <tbody>
            ${commits.map(commit => `
              <tr>
                <td>${escapeHtml(commit.commit)}</td>
                <td>${escapeHtml(commit.author)}</td>
                <td>${escapeHtml(commit.subject)}</td>
                <td>${commit.files_changed}</td>
              </tr>
            `).join('')}
          </tbody>
        </table>`;
    }

    load();
    setInterval(load, 10000);
  </script>
</body>
</html>"#;
    html.to_string()
}

