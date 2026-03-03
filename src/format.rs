use chrono::{DateTime, Utc};

pub fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn format_duration(ms: f64) -> String {
    let ms = ms as u64;
    if ms < 1000 {
        return format!("{}ms", ms);
    }
    let secs = ms / 1000;
    if secs < 60 {
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    let remain_secs = secs % 60;
    if mins < 60 {
        return format!("{}m {}s", mins, remain_secs);
    }
    let hours = mins / 60;
    let remain_mins = mins % 60;
    format!("{}h {}m", hours, remain_mins)
}

pub fn format_relative(date: &str) -> String {
    let parsed = DateTime::parse_from_rfc3339(date)
        .or_else(|_| DateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .map(|d| d.with_timezone(&Utc));

    let then = match parsed {
        Ok(d) => d,
        Err(_) => return date.to_string(),
    };

    let now = Utc::now();
    let diff = now.signed_duration_since(then);
    let mins = diff.num_minutes();

    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{}m ago", mins)
    } else {
        let hours = mins / 60;
        if hours < 24 {
            format!("{}h ago", hours)
        } else {
            let days = hours / 24;
            if days < 30 {
                format!("{}d ago", days)
            } else {
                format_date(date)
            }
        }
    }
}

pub fn format_date(date: &str) -> String {
    let parsed = DateTime::parse_from_rfc3339(date)
        .or_else(|_| DateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.fZ"));

    match parsed {
        Ok(d) => d.format("%b %-d, %Y").to_string(),
        Err(_) => date.to_string(),
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

// API pricing per million tokens: [input, output, cache_read]
fn model_pricing(model: &str) -> (f64, f64, f64) {
    let m = model.to_lowercase();
    if m.contains("opus") {
        (15.0, 75.0, 1.5)
    } else if m.contains("haiku") {
        (0.8, 4.0, 0.08)
    } else {
        // Default to sonnet
        (3.0, 15.0, 0.3)
    }
}

pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64, cache_read_tokens: u64) -> f64 {
    let (input_rate, output_rate, cache_rate) = model_pricing(model);
    let non_cache_input = if input_tokens > cache_read_tokens {
        input_tokens - cache_read_tokens
    } else {
        0
    };
    (non_cache_input as f64 * input_rate + output_tokens as f64 * output_rate + cache_read_tokens as f64 * cache_rate)
        / 1_000_000.0
}

pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        "<$0.01".to_string()
    } else {
        format!("${:.2}", cost)
    }
}

pub fn short_model(model: &str) -> String {
    if model.is_empty() {
        return String::new();
    }
    // Match patterns like "claude-opus-4-6", "claude-sonnet-4-5-20250929"
    let m = model.to_lowercase();
    let families = ["opus", "sonnet", "haiku"];

    for family in families {
        if let Some(idx) = m.find(family) {
            let name = format!(
                "{}{}",
                family[..1].to_uppercase(),
                &family[1..]
            );
            let rest = &m[idx + family.len()..];
            // Try to extract version numbers
            let parts: Vec<&str> = rest.split(|c: char| c == '-' || c == '_')
                .filter(|s| !s.is_empty())
                .collect();

            if parts.is_empty() {
                return name;
            }

            // First number is major version
            if let Ok(_major) = parts[0].parse::<u32>() {
                if parts.len() >= 2 {
                    if let Ok(_minor) = parts[1].parse::<u32>() {
                        // Skip if it looks like a date (8+ digits)
                        if parts[1].len() >= 8 {
                            return format!("{} {}", name, parts[0]);
                        }
                        return format!("{} {}.{}", name, parts[0], parts[1]);
                    }
                }
                return format!("{} {}", name, parts[0]);
            }

            return name;
        }
    }

    String::new()
}
