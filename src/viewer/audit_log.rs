pub fn show_audit_log(limit: usize) {
    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if let Err(error) = crate::auth::ensure_permission(&config, crate::auth::Permission::ReadAudit)
    {
        eprintln!("{error}");
        return;
    }

    match crate::audit::recent(&config, limit) {
        Ok(events) if events.is_empty() => println!("No audit events recorded yet."),
        Ok(events) => {
            println!("Recent audit events:\n");
            for event in events {
                println!(
                    "{}  {}  {}  {}  {}",
                    event.timestamp, event.actor, event.action, event.target, event.outcome
                );
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

pub fn prune_audit_log(days: Option<u32>) {
    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let user = match crate::auth::ensure_permission(&config, crate::auth::Permission::Administer) {
        Ok(user) => user,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let retain_days = days.unwrap_or(config.audit.retain_days);
    match crate::audit::prune(&config, retain_days) {
        Ok(removed) => {
            let _ = crate::audit::record(
                &config,
                &user.username,
                "audit.prune",
                "audit-log",
                "success",
                serde_json::json!({
                    "days": retain_days,
                    "removed": removed,
                }),
            );
            println!(
                "Pruned {} audit event(s) older than {} day(s).",
                removed, retain_days
            );
        }
        Err(error) => eprintln!("{error}"),
    }
}
