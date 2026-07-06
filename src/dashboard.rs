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
    let listener = TcpListener::bind(&address).map_err(|error| {
        AppError::message(format!("Failed to bind dashboard on {address}: {error}"))
    })?;

    println!("Gitwhisper dashboard running at http://{address}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = stream.set_nodelay(true);
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                std::thread::spawn(move || {
                    let _ = handle_connection(&mut stream);
                });
            }
            Err(error) => eprintln!("Dashboard connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(stream: &mut TcpStream) -> AppResult<()> {
    let mut buffer = [0; 16384];
    let read = stream.read(&mut buffer)?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    match path {
        "/" => respond_html(stream, DASHBOARD_HTML)?,
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
    respond(
        stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        "Not found",
    )
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) -> AppResult<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, OPTIONS\r\n\
         Content-Security-Policy: default-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com https://fonts.googleapis.com https://fonts.gstatic.com;\r\n\r\n\
         {}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

pub const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="color-scheme" content="dark" />
  <title>GitWhisper Intelligence Dashboard</title>
  <meta name="description" content="GitWhisper — AI-powered commit intelligence. Understand file evolution, ownership risk, and team delivery patterns." />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet" />
  <script src="https://cdnjs.cloudflare.com/ajax/libs/Chart.js/4.4.1/chart.umd.min.js" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
  <style>
    :root {
      --bg:          #05080f;
      --card:        rgba(22, 27, 34, 0.6);
      --card-border: rgba(48, 54, 61, 0.8);
      --text:        #e6edf3;
      --muted:       #8b949e;
      --subtle:      #484f58;
      --accent:      #238636;
      --accent-2:    #2ea043;
      --danger:      #f85149;
      --warn:        #d29922;
      --warn-bg:     rgba(210,153,34,0.15);
      --blue:        #58a6ff;
      --blue-bg:     rgba(88,166,255,0.12);
      --purple:      #a371f7;
      --mono:        'JetBrains Mono', ui-monospace, SFMono-Regular, Consolas, monospace;
      --ui:          'Inter', ui-sans-serif, system-ui, -apple-system, sans-serif;
      --shadow-card: 0 12px 40px rgba(0,0,0,0.5);
      --radius:      16px;
    }

    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    html { scroll-behavior: smooth; }

    body {
      min-height: 100vh;
      background-color: var(--bg);
      background-image:
        radial-gradient(circle at 15% 0%, rgba(35,134,54,0.15) 0%, transparent 40%),
        radial-gradient(circle at 85% 100%, rgba(88,166,255,0.1) 0%, transparent 40%),
        radial-gradient(circle at 50% 50%, rgba(163,113,247,0.05) 0%, transparent 60%);
      background-attachment: fixed;
      color: var(--text);
      font-family: var(--ui);
      overflow-x: hidden;
    }

    /* Dot-grid texture */
    body::before {
      content: '';
      position: fixed;
      inset: 0;
      z-index: -1;
      pointer-events: none;
      background-image: radial-gradient(circle, rgba(255,255,255,0.05) 1px, transparent 1px);
      background-size: 24px 24px;
      mask-image: linear-gradient(to bottom, rgba(0,0,0,1) 0%, transparent 100%);
    }

    a { color: inherit; text-decoration: none; }
    button, input { font: inherit; }

    .app {
      width: min(1500px, calc(100% - 48px));
      margin: 0 auto;
      padding: 20px 0 80px;
    }

    /* ── NAVBAR ─────────────────────────────────────── */
    .navbar {
      position: sticky;
      top: 12px;
      z-index: 50;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 12px 24px;
      background: rgba(13, 17, 23, 0.75);
      border: 1px solid var(--card-border);
      border-radius: 20px;
      backdrop-filter: blur(32px) saturate(150%);
      box-shadow: 0 4px 24px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.05);
    }

    .brand { display: flex; align-items: center; gap: 16px; min-width: 0; }

    .logo-mark {
      display: grid; place-items: center;
      width: 44px; height: 44px; flex-shrink: 0;
      border-radius: 12px;
      border: 1px solid rgba(35,134,54,0.5);
      background: linear-gradient(135deg, rgba(35,134,54,0.4), rgba(35,134,54,0.1));
      box-shadow: 0 0 20px rgba(35,134,54,0.3), inset 0 1px 0 rgba(255,255,255,0.15);
      font-size: 22px;
    }

    .brand-title { 
      font-size: 1.25rem; font-weight: 900; letter-spacing: -0.04em; 
      background: linear-gradient(90deg, #fff, #8b949e);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }
    .brand-sub {
      font-family: var(--mono); font-size: 0.75rem; color: var(--muted); margin-top: 2px;
    }

    .nav-right { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }

    .health-badge {
      display: inline-flex; align-items: center; gap: 8px;
      padding: 6px 14px; border-radius: 999px;
      background: rgba(22, 27, 34, 0.8);
      border: 1px solid var(--card-border);
      box-shadow: inset 0 1px 0 rgba(255,255,255,0.05);
      font-size: 0.8rem; font-weight: 700;
      backdrop-filter: blur(10px);
    }
    .health-score {
      font-family: var(--mono);
      font-size: 1rem;
      font-weight: 900;
    }
    .health-A { color: #39d353; text-shadow: 0 0 10px rgba(57,211,83,0.5); }
    .health-B { color: #d29922; text-shadow: 0 0 10px rgba(210,153,34,0.5); }
    .health-C { color: #f85149; text-shadow: 0 0 10px rgba(248,81,73,0.5); }

    .btn {
      display: inline-flex; align-items: center; gap: 6px;
      padding: 8px 16px; border-radius: 10px; border: 1px solid var(--card-border);
      background: rgba(22, 27, 34, 0.8); color: var(--text);
      font-size: 0.85rem; font-weight: 600; cursor: pointer;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
      backdrop-filter: blur(10px);
    }
    .btn:hover { 
      border-color: rgba(88,166,255,0.5); 
      background: rgba(88,166,255,0.1); 
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(88,166,255,0.15);
    }
    .btn-accent { border-color: rgba(35,134,54,0.5); background: rgba(35,134,54,0.15); color: #3fb950; }
    .btn-accent:hover { background: rgba(35,134,54,0.25); box-shadow: 0 4px 12px rgba(35,134,54,0.2); }

    /* ── SECTION TITLES ──────────────────────────────── */
    .section-title {
      font-size: 1.1rem; font-weight: 800; letter-spacing: -0.02em; color: var(--text);
      margin: 32px 0 16px; display: flex; align-items: center; gap: 10px;
    }
    .section-title::before {
      content: ''; display: block; width: 4px; height: 18px; border-radius: 2px;
      background: linear-gradient(180deg, var(--accent), var(--blue));
    }

    /* ── CARD ────────────────────────────────────────── */
    .card {
      background: var(--card); border: 1px solid var(--card-border);
      border-radius: var(--radius); box-shadow: var(--shadow-card);
      backdrop-filter: blur(20px) saturate(120%);
      transition: transform 0.3s, box-shadow 0.3s, border-color 0.3s;
      position: relative; overflow: hidden;
    }
    .card::after {
      content: ''; position: absolute; inset: 0; pointer-events: none;
      border-radius: var(--radius); box-shadow: inset 0 1px 0 rgba(255,255,255,0.05);
    }
    .card:hover { border-color: rgba(88,166,255,0.3); box-shadow: 0 16px 48px rgba(0,0,0,0.6), 0 0 0 1px rgba(88,166,255,0.1); }
    
    .card-header {
      display: flex; align-items: flex-start; justify-content: space-between;
      gap: 12px; padding: 20px 24px 0;
    }
    .card-title { font-size: 0.95rem; font-weight: 800; letter-spacing: -0.02em; color: var(--text); }
    .card-sub   { font-size: 0.8rem; color: var(--muted); margin-top: 4px; }
    .card-badge {
      flex-shrink: 0; padding: 4px 12px; border-radius: 999px;
      border: 1px solid var(--card-border); font-family: var(--mono);
      font-size: 0.7rem; font-weight: 700; color: var(--muted);
      background: rgba(1, 4, 9, 0.5); white-space: nowrap;
    }
    .card-body { padding: 20px 24px 24px; }

    /* ── TIER 1: STATS & INTENT ──────────────────────── */
    .overview-grid {
      display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); gap: 16px; margin-top: 24px;
    }
    .stats-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; }
    
    .stat-card {
      position: relative; padding: 24px;
      background: var(--card); border: 1px solid var(--card-border);
      border-radius: var(--radius); box-shadow: var(--shadow-card);
      backdrop-filter: blur(20px);
      display: flex; flex-direction: column; justify-content: space-between;
    }
    .stat-card::before {
      content: ''; position: absolute; top: -20px; right: -20px;
      width: 100px; height: 100px; border-radius: 50%;
      background: var(--glow, rgba(35,134,54,0.12));
      filter: blur(30px); pointer-events: none;
    }
    .stat-label {
      font-family: var(--mono); font-size: 0.75rem; font-weight: 700;
      letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted); margin-bottom: 16px;
    }
    .stat-value {
      font-size: clamp(2.5rem, 4vw, 3.5rem); font-weight: 900; letter-spacing: -0.05em;
      line-height: 1; color: var(--text); font-family: var(--mono);
      background: var(--gradient, linear-gradient(90deg, #fff, #8b949e));
      -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    }
    .stat-sub { margin-top: 12px; font-size: 0.8rem; color: var(--muted); line-height: 1.4; font-weight: 500; }
    .stat-sub strong { color: var(--text); }

    .intent-chart-wrap { position: relative; height: 220px; display: flex; align-items: center; justify-content: center; }

    /* ── TIER 2: VELOCITY ────────────────────────────── */
    .velocity-grid {
      display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1.5fr); gap: 16px;
    }

    /* ── HEATMAP ───────────────────────────────────── */
    .hm-cell {
      width: 14px; height: 14px; border-radius: 4px;
      background: rgba(22, 27, 34, 0.8); border: 1px solid rgba(48,54,61,0.6);
      cursor: default; transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1); position: relative;
    }
    .hm-cell:hover { transform: scale(1.6); z-index: 10; box-shadow: 0 4px 12px rgba(0,0,0,0.5); border-color: rgba(255,255,255,0.3); }
    .hm-cell[data-level="1"] { background: #0e4429; border-color: rgba(35,134,54,0.3); }
    .hm-cell[data-level="2"] { background: #006d32; border-color: rgba(35,134,54,0.5); }
    .hm-cell[data-level="3"] { background: #26a641; border-color: rgba(35,134,54,0.7); }
    .hm-cell[data-level="4"] { background: #39d353; border-color: rgba(57,211,83,0.8); box-shadow: 0 0 10px rgba(57,211,83,0.4); }

    .hm-legend { display: flex; align-items: center; gap: 6px; margin-top: 12px; justify-content: flex-end; }
    .hm-legend-label { font-family: var(--mono); font-size: 0.65rem; color: var(--subtle); font-weight: 700; text-transform: uppercase; }
    .hm-legend-cell  { width: 12px; height: 12px; border-radius: 3px; }

    /* ── COMMIT FEED ───────────────────────────────── */
    .commit-feed {
      display: flex; flex-direction: column; gap: 4px;
      max-height: 380px; overflow-y: auto; padding-right: 4px;
      scrollbar-color: rgba(48,54,61,0.8) transparent; scrollbar-width: thin;
    }
    .commit-item {
      display: grid; grid-template-columns: auto minmax(0,1fr); gap: 14px;
      align-items: center; padding: 12px 16px;
      background: rgba(1, 4, 9, 0.3);
      border: 1px solid rgba(48,54,61,0.4); border-left: 4px solid transparent; border-radius: 12px;
      transition: transform 0.2s, background 0.2s, border-color 0.2s; cursor: default;
    }
    .commit-item:hover { background: rgba(48,54,61,0.4); transform: translateX(4px); }
    .commit-item.intent-feat     { border-left-color: var(--accent); }
    .commit-item.intent-fix      { border-left-color: var(--danger); }
    .commit-item.intent-refactor { border-left-color: var(--blue); }
    .commit-item.intent-docs     { border-left-color: var(--purple); }
    .commit-item.intent-perf     { border-left-color: var(--warn); }
    .commit-item.intent-other    { border-left-color: var(--subtle); }

    .commit-avatar {
      width: 36px; height: 36px; border-radius: 50%; display: grid; place-items: center;
      font-family: var(--mono); font-size: 0.85rem; font-weight: 800; color: #fff;
      border: 2px solid rgba(255,255,255,0.1); box-shadow: 0 4px 10px rgba(0,0,0,0.3);
    }
    .commit-body { min-width: 0; }
    .commit-subject { font-size: 0.9rem; font-weight: 600; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .commit-meta-row { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 6px; align-items: center; }
    .commit-sha    { font-family: var(--mono); font-size: 0.72rem; color: var(--warn); font-weight: 700; background: var(--warn-bg); padding: 2px 6px; border-radius: 4px; }
    .commit-author { font-family: var(--mono); font-size: 0.75rem; font-weight: 600; color: var(--blue); }
    .commit-time   { font-size: 0.72rem; color: var(--muted); }

    /* ── TIER 3: TEAM & HEALTH ─────────────────────── */
    .team-grid {
      display: grid; grid-template-columns: minmax(0, 1.2fr) minmax(0, 2fr); gap: 16px;
    }

    /* ── RISKS ─────────────────────────────────────── */
    .risks-grid { display: grid; grid-template-columns: repeat(auto-fill,minmax(300px,1fr)); gap: 16px; margin-bottom: 24px;}
    .risk-card {
      padding: 18px; border-radius: 12px;
      border: 1px solid var(--card-border); border-left-width: 4px;
      background: rgba(1, 4, 9, 0.4); box-shadow: 0 4px 12px rgba(0,0,0,0.2);
      transition: transform 0.2s, box-shadow 0.2s;
    }
    .risk-card:hover { transform: translateY(-4px); box-shadow: 0 12px 24px rgba(0,0,0,0.4); }
    .risk-card.kind-silo    { border-left-color: #f0883e; background: linear-gradient(135deg, rgba(240,136,62,0.1), transparent); }
    .risk-card.kind-broad   { border-left-color: var(--warn); background: linear-gradient(135deg, rgba(210,153,34,0.1), transparent); }
    .risk-card.kind-default { border-left-color: var(--muted); }

    .risk-card-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 10px; }
    .risk-kind-badge {
      flex-shrink: 0; font-family: var(--mono); font-size: 0.65rem; font-weight: 800;
      letter-spacing: 0.05em; text-transform: uppercase; padding: 4px 10px; border-radius: 999px;
    }
    .kind-silo .risk-kind-badge    { background: rgba(240,136,62,0.2); color: #f0883e; border: 1px solid rgba(240,136,62,0.4); }
    .kind-broad .risk-kind-badge   { background: var(--warn-bg); color: var(--warn); border: 1px solid rgba(210,153,34,0.4); }
    .kind-default .risk-kind-badge { background: rgba(139,148,158,0.2); color: var(--muted); border: 1px solid rgba(139,148,158,0.3); }
    .risk-subject { font-family: var(--mono); font-size: 0.85rem; font-weight: 700; color: var(--text); word-break: break-all; margin: 0; }
    .risk-detail  { font-size: 0.85rem; color: var(--muted); margin-top: 6px; line-height: 1.5; }

    /* ── TABLES (PEOPLE & FILES) ───────────────────── */
    .modern-table { width: 100%; border-collapse: collapse; }
    .modern-table th {
      font-family: var(--mono); font-size: 0.7rem; font-weight: 800;
      letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted);
      padding: 12px 16px; text-align: left; border-bottom: 1px solid rgba(255,255,255,0.1);
      background: rgba(1, 4, 9, 0.4); position: sticky; top: 0; z-index: 1;
    }
    .modern-table td { padding: 12px 16px; font-size: 0.85rem; border-bottom: 1px solid rgba(255,255,255,0.05); vertical-align: middle; }
    .modern-table tbody tr { transition: background 0.2s; }
    .modern-table tbody tr:hover { background: rgba(255,255,255,0.03); }
    .modern-table tbody tr:last-child td { border-bottom: none; }
    
    .author-cell { display: flex; align-items: center; gap: 10px; font-weight: 600; }
    .author-avatar { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; font-family: var(--mono); font-size: 0.7rem; font-weight: 800; color: #fff; flex-shrink: 0; border: 1px solid rgba(255,255,255,0.1); }
    
    .bar-row { display: flex; align-items: center; gap: 10px; }
    .bar-num  { font-family: var(--mono); font-size: 0.85rem; font-weight: 700; min-width: 32px; }
    .bar-track { flex: 1; height: 6px; background: rgba(0,0,0,0.5); border-radius: 4px; overflow: hidden; min-width: 80px; box-shadow: inset 0 1px 3px rgba(0,0,0,0.5); }
    .bar-fill  { height: 100%; border-radius: 4px; box-shadow: 0 0 10px rgba(57,211,83,0.5); }

    /* ── UTILS ─────────────────────────────────────── */
    .empty-state { display: grid; place-items: center; min-height: 160px; color: var(--muted); font-size: 0.9rem; text-align: center; padding: 24px; border: 1px dashed var(--card-border); border-radius: 12px; }
    .data-fade { transition: opacity 0.4s ease, transform 0.4s ease; }
    .data-fade.loading { opacity: 0.5; transform: scale(0.99); }
    .sr-only { position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0; }
  </style>
</head>
<body>
  <div class="app">
    <!-- NAVBAR -->
    <nav class="navbar" aria-label="GitWhisper dashboard navigation">
      <div class="brand">
        <div class="logo-mark" aria-hidden="true">🔮</div>
        <div>
          <div class="brand-title">GitWhisper</div>
          <div class="brand-sub">Systematic Intelligence Dashboard</div>
        </div>
      </div>
      <div class="nav-right">
        <div class="health-badge" id="healthBadge">
          Health: <span class="health-score" id="healthScore">...</span>
        </div>
        <button class="btn btn-accent" id="refreshBtn" type="button">↺ Refresh Data</button>
      </div>
    </nav>

    <div class="data-fade" id="mainContent">

      <h2 class="section-title">Executive Overview</h2>
      
      <!-- TIER 1: STATS & INTENT -->
      <section class="overview-grid">
        <div class="stats-grid">
          <article class="stat-card" style="--glow: rgba(57,211,83,0.15)">
            <div class="stat-label">Total Commits</div>
            <div class="stat-value" id="statCommits" style="--gradient: linear-gradient(135deg, #39d353, #238636)">—</div>
            <div class="stat-sub" id="statCommitsSub">Loading…</div>
          </article>
          <article class="stat-card" style="--glow: rgba(88,166,255,0.15)">
            <div class="stat-label">Unique Authors</div>
            <div class="stat-value" id="statAuthors" style="--gradient: linear-gradient(135deg, #58a6ff, #1f6feb)">—</div>
            <div class="stat-sub" id="statAuthorsSub">Loading…</div>
          </article>
          <article class="stat-card" style="--glow: rgba(163,113,247,0.15)">
            <div class="stat-label">Files Touched</div>
            <div class="stat-value" id="statFiles" style="--gradient: linear-gradient(135deg, #a371f7, #8957e5)">—</div>
            <div class="stat-sub" id="statFilesSub">Loading…</div>
          </article>
          <article class="stat-card" style="--glow: rgba(210,153,34,0.15)">
            <div class="stat-label">30-Day Velocity</div>
            <div class="stat-value" id="statActivity" style="--gradient: linear-gradient(135deg, #d29922, #9e6a03)">—</div>
            <div class="stat-sub" id="statActivitySub">Loading…</div>
          </article>
        </div>
        
        <article class="card">
          <div class="card-header">
            <div>
              <div class="card-title">Intent Breakdown</div>
              <div class="card-sub">Engineering effort distribution</div>
            </div>
          </div>
          <div class="card-body">
            <div class="intent-chart-wrap">
              <canvas id="intentChart"></canvas>
            </div>
          </div>
        </article>
      </section>

      <h2 class="section-title">Velocity & Timeline</h2>

      <!-- TIER 2: VELOCITY -->
      <section class="velocity-grid">
        <!-- 90-Day Commit Heatmap -->
        <article class="card" aria-labelledby="heatmapTitle">
          <div class="card-header">
            <div>
              <div class="card-title" id="heatmapTitle">90-Day Commit Heatmap</div>
              <div class="card-sub">Daily commit frequency — last 13 weeks</div>
            </div>
            <span class="card-badge" id="heatmapTotal">0 commits</span>
          </div>
          <div class="card-body">
            <div class="hm-legend" style="margin-bottom:14px;">
              <span class="hm-legend-label">Less</span>
              <div class="hm-legend-cell" style="background:rgba(22,27,34,0.8);border:1px solid rgba(48,54,61,0.6);"></div>
              <div class="hm-legend-cell" style="background:#0e4429;"></div>
              <div class="hm-legend-cell" style="background:#006d32;"></div>
              <div class="hm-legend-cell" style="background:#26a641;"></div>
              <div class="hm-legend-cell" style="background:#39d353;"></div>
              <span class="hm-legend-label">More</span>
            </div>
            <div id="heatmapGrid" style="overflow-x:auto;"></div>
          </div>
        </article>

        <!-- Recent Commits Feed -->
        <article class="card" aria-labelledby="feedTitle">
          <div class="card-header">
            <div>
              <div class="card-title" id="feedTitle">Recent Commits</div>
              <div class="card-sub">Latest changes with semantic intent</div>
            </div>
          </div>
          <div class="card-body" style="padding-top:16px; padding-right:12px;">
            <div class="commit-feed" id="commitFeed">
              <div class="empty-state">Loading commits…</div>
            </div>
          </div>
        </article>
      </section>

      <h2 class="section-title">Code Health & Team</h2>
      
      <!-- RISKS -->
      <div class="risks-grid" id="risksGrid">
        <div class="empty-state" style="grid-column: 1 / -1">Loading health signals...</div>
      </div>

      <!-- TIER 3: TEAM -->
      <section class="team-grid">
        <!-- Top People -->
        <article class="card">
          <div class="card-header">
            <div>
              <div class="card-title">Top Contributors</div>
              <div class="card-sub">Ranked by commit volume</div>
            </div>
          </div>
          <div class="card-body" style="padding: 16px 0 0 0;">
            <div id="peopleTableWrap" style="overflow-x:auto;"></div>
          </div>
        </article>

        <!-- File Churn Chart -->
        <article class="card">
          <div class="card-header">
            <div>
              <div class="card-title">Top File Churn</div>
              <div class="card-sub">Highest frequency modified files</div>
            </div>
          </div>
          <div class="card-body">
            <div style="position: relative; height: 320px;">
              <canvas id="churnChart"></canvas>
            </div>
          </div>
        </article>
      </section>

    </div><!-- /mainContent -->
  </div>

  <script>
    if (window.Chart) {
      Chart.defaults.color = '#8b949e';
      Chart.defaults.borderColor = 'rgba(255,255,255,0.05)';
      Chart.defaults.font.family = "'Inter', sans-serif";
    }

    const state = {
      snapshot: null,
      intentChart: null,
      churnChart: null,
      firstLoad: true
    };
    const $ = id => document.getElementById(id);

    function escapeHtml(v) { return String(v ?? '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }
    function fmt(n) { const v=Number(n); return isFinite(v) ? new Intl.NumberFormat().format(Math.round(v)) : '0'; }
    function shortPath(p, segs = 2) { return String(p || '').split(/[\\/]/).slice(-segs).join('/'); }
    function hashHue(s) { let h=0; for(let i=0;i<s.length;i++) h=s.charCodeAt(i)+((h<<5)-h); return Math.abs(h)%360; }
    function avatarStyle(name) { const hue=hashHue(String(name||'?')); return `background:linear-gradient(135deg,hsl(${hue},70%,50%),hsl(${(hue+40)%360},80%,30%))`; }
    function inferIntent(msg) {
      const t = String(msg||'').toLowerCase();
      if (/^feat[:(]|^add |implement|introduce/.test(t)) return 'feat';
      if (/^fix[:(]|bug|hotfix|patch|resolve/.test(t))  return 'fix';
      if (/^refactor[:(]|cleanup|rework|simplify/.test(t)) return 'refactor';
      if (/^docs[:(]|readme|comment|documentation/.test(t)) return 'docs';
      if (/^perf[:(]|optimize|speed|cache/.test(t)) return 'perf';
      return 'other';
    }

    function calculateHealth(snap) {
      const risks = Array.isArray(snap.risks) ? snap.risks.length : 0;
      const score = Math.max(0, 100 - (risks * 15));
      const el = $('healthScore');
      el.textContent = score >= 90 ? 'A+' : score >= 75 ? 'B' : score >= 50 ? 'C' : 'D';
      el.className = 'health-score ' + (score >= 90 ? 'health-A' : score >= 75 ? 'health-B' : 'health-C');
    }

    function renderStats(snap) {
      const ov = snap.overview || {};
      $('statCommits').textContent = fmt(ov.total_commits);
      $('statCommitsSub').innerHTML = `<strong>${fmt(ov.commits_last_7d)}</strong> this week`;
      $('statAuthors').textContent = fmt(ov.unique_authors);
      $('statAuthorsSub').textContent = `active contributors`;
      $('statFiles').textContent = fmt(ov.files_touched);
      $('statFilesSub').textContent = `total files tracked`;
      $('statActivity').textContent = fmt(ov.commits_last_30d);
      $('statActivitySub').innerHTML = `commits in last 30d`;
    }

    function renderIntentChart(snap) {
      if (!window.Chart) return;
      const ib = snap.intent_breakdown || { feature:0, fix:0, refactor:0, docs:0, performance:0, unknown:0 };
      const labels = ['Features', 'Fixes', 'Refactor', 'Docs', 'Perf', 'Other'];
      const data = [ib.feature||0, ib.fix||0, ib.refactor||0, ib.docs||0, ib.performance||0, ib.unknown||0];
      
      const sum = data.reduce((a,b)=>a+b,0);
      if(sum === 0) {
         const wrap = $('intentChart').parentElement;
         wrap.innerHTML = '<div class="empty-state" style="min-height:100px;border:none;">No semantic intent data yet.</div>';
         return;
      }

      const colors = ['#39d353', '#f85149', '#58a6ff', '#a371f7', '#d29922', '#484f58'];
      
      if (state.intentChart) state.intentChart.destroy();
      state.intentChart = new Chart($('intentChart'), {
        type: 'doughnut',
        data: { labels, datasets: [{ data, backgroundColor: colors, borderWidth: 0, hoverOffset: 8 }] },
        options: {
          responsive: true, maintainAspectRatio: false, cutout: '75%',
          plugins: { legend: { position: 'right', labels: { usePointStyle: true, padding: 16, font: { size: 11 } } } }
        }
      });
    }

    function renderHeatmap(snap) {
      const recent = Array.isArray(snap.recent_commits) ? snap.recent_commits : [];
      const weekly = Array.isArray(snap.weekly_activity) ? snap.weekly_activity : [];
      const countByDate = new Map();
      
      recent.forEach(c => {
        const d = new Date(c.timestamp||c.date||'');
        if(!isNaN(d)) countByDate.set(d.toISOString().slice(0,10), (countByDate.get(d.toISOString().slice(0,10))||0)+1);
      });
      weekly.forEach(w => {
        if(!w.week) return;
        const [year, wkStr] = w.week.split('-W');
        if(!year || !wkStr) return;
        const jan4 = new Date(Number(year), 0, 4);
        const monday = new Date(jan4.getTime() - (jan4.getDay()||7-1)*86400000 + (Number(wkStr)-1)*7*86400000);
        for(let i=0; i<7; i++){
          const key = new Date(monday.getTime()+i*86400000).toISOString().slice(0,10);
          if(!countByDate.has(key)) countByDate.set(key, Math.round((w.commits||0)/7));
        }
      });

      const today = new Date(); today.setHours(0,0,0,0);
      const startDay = new Date(today); startDay.setDate(startDay.getDate()-90); startDay.setDate(startDay.getDate()-startDay.getDay());

      const cells = [], weekLabels = [];
      let d = new Date(startDay);
      while (d <= today || cells.length % 7 !== 0) {
        if(d.getDay()===0) weekLabels.push(d.toLocaleDateString(undefined,{month:'short',day:'numeric'}));
        const key = d.toISOString().slice(0,10);
        const count = countByDate.get(key)||0;
        cells.push({ date: key, count, level: count>9?4:count>5?3:count>2?2:count>0?1:0, future: d>today });
        d = new Date(d.getTime()+86400000);
      }

      $('heatmapTotal').textContent = `${fmt([...countByDate.values()].reduce((a,b)=>a+b,0))} commits`;
      
      let html = `<div style="display:flex;gap:0;"><div style="display:grid;grid-template-rows:16px repeat(7,14px);gap:4px;margin-right:8px;"><div></div>`;
      ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'].forEach(dn => {
        html += `<div style="font-family:var(--mono);font-size:0.6rem;color:var(--subtle);line-height:14px;text-align:right;">${dn[0]}</div>`;
      });
      html += `</div><div style="overflow-x:auto;padding-bottom:10px;"><div style="display:flex;gap:4px;">`;
      
      const numWeeks = Math.ceil(cells.length/7);
      for(let w=0; w<numWeeks; w++){
        html += `<div style="display:flex;flex-direction:column;gap:4px;">`;
        const label = w < weekLabels.length ? weekLabels[w] : '';
        html += `<div style="height:16px;font-family:var(--mono);font-size:0.65rem;color:var(--muted);">${w%2===0&&label?escapeHtml(label):''}</div>`;
        for(let day=0; day<7; day++){
          const idx = w*7+day;
          if(idx>=cells.length){ html+=`<div style="width:14px;height:14px;"></div>`; continue; }
          const cell = cells[idx];
          if(cell.future){ html+=`<div style="width:14px;height:14px;border-radius:4px;background:transparent;"></div>`; continue; }
          html += `<div class="hm-cell" data-level="${cell.level}" title="${cell.date}: ${cell.count} commits"></div>`;
        }
        html += `</div>`;
      }
      $('heatmapGrid').innerHTML = html + `</div></div></div>`;
    }

    function renderCommitFeed(snap) {
      const commits = Array.isArray(snap.recent_commits) ? snap.recent_commits.slice(0,10) : [];
      if(!commits.length) { $('commitFeed').innerHTML = '<div class="empty-state">No recent commits.</div>'; return; }
      
      $('commitFeed').innerHTML = commits.map(c => {
        const sha = String(c.commit||'').slice(0,7);
        const msg = String(c.subject||'');
        const intent = inferIntent(msg);
        return `
          <div class="commit-item intent-${intent}">
            <div class="commit-avatar" style="${avatarStyle(c.author)}">${escapeHtml(String(c.author||'?')[0].toUpperCase())}</div>
            <div class="commit-body">
              <div class="commit-subject" title="${escapeHtml(msg)}">${escapeHtml(msg)}</div>
              <div class="commit-meta-row">
                <span class="commit-sha">${escapeHtml(sha)}</span>
                <span class="commit-author">${escapeHtml(c.author)}</span>
                ${c.files_changed ? `<span style="font-size:0.7rem;color:var(--muted);font-family:var(--mono);">${c.files_changed} files</span>` : ''}
              </div>
            </div>
          </div>
        `;
      }).join('');
    }

    function renderPeople(snap) {
      const people = Array.isArray(snap.people) ? snap.people.slice(0,10) : [];
      if(!people.length) { $('peopleTableWrap').innerHTML = '<div class="empty-state" style="border:none;">No contributor data.</div>'; return; }
      const maxC = Math.max(1, ...people.map(p=>Number(p.commits||0)));
      $('peopleTableWrap').innerHTML = `<table class="modern-table">
        <thead><tr><th>Author</th><th>Commits</th><th>Files</th></tr></thead><tbody>
        ${people.map(p => {
          const pct = Math.max(0,Math.min(100,(Number(p.commits)/maxC)*100));
          return `<tr>
            <td>
              <div class="author-cell">
                <div class="author-avatar" style="${avatarStyle(p.author)}">${escapeHtml(String(p.author||'?')[0].toUpperCase())}</div>
                ${escapeHtml(p.author)}
              </div>
            </td>
            <td>
              <div class="bar-row">
                <span class="bar-num">${fmt(p.commits)}</span>
                <div class="bar-track"><div class="bar-fill" style="width:${pct}%;background:linear-gradient(90deg,var(--accent),#39d353);"></div></div>
              </div>
            </td>
            <td style="font-family:var(--mono);color:var(--muted);">${fmt(p.files_touched)}</td>
          </tr>`;
        }).join('')}</tbody></table>`;
    }

    function renderChurnChart(snap) {
      if (!window.Chart) return;
      const files = Array.isArray(snap.files) ? snap.files.slice().sort((a,b)=>Number(b.commits)-Number(a.commits)).slice(0,10) : [];
      if(!files.length) return;
      const labels = files.map(f => shortPath(f.path, 2));
      const data = files.map(f => Number(f.commits||0));
      
      const ctx = $('churnChart');
      if(state.churnChart) state.churnChart.destroy();
      state.churnChart = new Chart(ctx, {
        type: 'bar',
        data: { labels, datasets: [{ data, backgroundColor: 'rgba(88,166,255,0.8)', borderRadius: 6 }] },
        options: {
          indexAxis: 'y', responsive: true, maintainAspectRatio: false,
          scales: {
            x: { grid: { color: 'rgba(255,255,255,0.05)' } },
            y: { grid: { display: false }, ticks: { font: { family: "'JetBrains Mono'" } } }
          },
          plugins: { legend: { display: false } }
        }
      });
    }

    function renderRisks(snap) {
      const risks = Array.isArray(snap.risks) ? snap.risks : [];
      if(!risks.length) { $('risksGrid').innerHTML = '<div class="empty-state" style="grid-column:1/-1;">\u2705 No repository risks detected.</div>'; return; }
      
      $('risksGrid').innerHTML = risks.map(r => {
        const k = String(r.kind||'').toLowerCase();
        const kcls = k.includes('silo') ? 'kind-silo' : k.includes('broad') ? 'kind-broad' : 'kind-default';
        return `
          <div class="risk-card ${kcls}">
            <div class="risk-card-top">
              <p class="risk-subject">${escapeHtml(r.subject)}</p>
              <span class="risk-kind-badge">${escapeHtml(r.kind)}</span>
            </div>
            <p class="risk-detail">${escapeHtml(r.detail)}</p>
          </div>
        `;
      }).join('');
    }

    async function loadSnapshot() {
      try {
        const res = await fetch('/snapshot.json', { cache: 'no-store' });
        if(!res.ok) throw new Error();
        const snap = await res.json();
        
        calculateHealth(snap);
        renderStats(snap);
        renderIntentChart(snap);
        renderHeatmap(snap);
        renderCommitFeed(snap);
        renderPeople(snap);
        renderChurnChart(snap);
        renderRisks(snap);
        
        $('mainContent').classList.remove('loading');
      } catch (err) {
        console.error(err);
      }
    }

    $('refreshBtn').addEventListener('click', () => {
      $('mainContent').classList.add('loading');
      loadSnapshot();
    });
    
    loadSnapshot();
  </script>
</body>
</html>
"##;
