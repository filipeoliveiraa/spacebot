//! Pure detection heuristics for captcha / login wall / WAF block pages.
//!
//! Runs against post-navigation page state (HTML + final URL). DOM-only for
//! v1 — chromiumoxide response-header inspection is a separate workstream.
//! Detection is conservative: false positives on legitimate Cloudflare-hosted
//! pages would short-circuit valid worker runs, so each heuristic requires a
//! specific signal rather than a generic vendor marker.

use crate::agent::worker::{BlockEvidence, BlockReason};

/// Result of running detection over a page snapshot.
#[derive(Debug, Clone)]
pub struct Detection {
    pub reason: BlockReason,
    pub evidence: BlockEvidence,
}

/// Inspect the page snapshot and return a detection if any heuristic fires.
/// Caller passes the raw page HTML (full content), the final URL after any
/// redirects, and optionally a navigation target host so the login-wall path
/// detector can avoid false-positive on intentional logins.
pub fn classify(
    html: &str,
    final_url: Option<&str>,
    navigation_target_host: Option<&str>,
) -> Option<Detection> {
    if let Some(reason) = detect_captcha(html) {
        return Some(Detection {
            reason,
            evidence: build_evidence(html, final_url),
        });
    }
    if let Some(reason) = detect_cloudflare_challenge(html) {
        return Some(Detection {
            reason,
            evidence: build_evidence(html, final_url),
        });
    }
    if let Some(reason) = detect_login_wall(final_url, navigation_target_host) {
        return Some(Detection {
            reason,
            evidence: build_evidence(html, final_url),
        });
    }
    None
}

/// Detect a captcha challenge by scanning for known provider iframe srcs and
/// form-input markers. Order matters — provider-specific markers come first
/// so the `provider` field in `BlockReason::Captcha` is precise.
pub fn detect_captcha(html: &str) -> Option<BlockReason> {
    if html.contains("challenges.cloudflare.com/turnstile/") {
        return Some(BlockReason::Captcha {
            provider: "cloudflare-turnstile".to_string(),
        });
    }
    if html.contains("hcaptcha.com/captcha/") || html.contains("js.hcaptcha.com/1/api.js") {
        return Some(BlockReason::Captcha {
            provider: "hcaptcha".to_string(),
        });
    }
    if html.contains("google.com/recaptcha/")
        || html.contains("gstatic.com/recaptcha/")
        || html.contains("g-recaptcha-response")
    {
        return Some(BlockReason::Captcha {
            provider: "recaptcha".to_string(),
        });
    }
    None
}

/// Detect a Cloudflare interstitial challenge page (the "checking your
/// browser before accessing" interstitial, distinct from sites merely fronted
/// by Cloudflare). Anchored on body text + a CF-specific cookie/header marker
/// to avoid firing on every CF-hosted site.
pub fn detect_cloudflare_challenge(html: &str) -> Option<BlockReason> {
    let interstitial_markers = [
        "Checking your browser before accessing",
        "challenge-platform",
        "cf-mitigated",
        "cf_chl_opt",
    ];
    let mut matched = 0;
    for marker in interstitial_markers.iter() {
        if html.contains(marker) {
            matched += 1;
        }
    }
    if matched >= 2 {
        return Some(BlockReason::FraudDetect {
            vendor: "cloudflare".to_string(),
        });
    }
    None
}

/// Detect that the navigation landed on a login URL distinct from the
/// requested host's intended page. We require either a path that explicitly
/// signals login, or a same-host redirect into a login path. Without a
/// requested-target host, we only fire on the explicit signals.
pub fn detect_login_wall(
    final_url: Option<&str>,
    navigation_target_host: Option<&str>,
) -> Option<BlockReason> {
    let url = final_url?;
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path().to_lowercase();
    let login_path_signals = [
        "/login",
        "/signin",
        "/auth/",
        "/account/login",
        "/users/sign_in",
    ];
    let path_is_login = login_path_signals
        .iter()
        .any(|signal| path.contains(signal));
    if !path_is_login {
        return None;
    }
    if let Some(target_host) = navigation_target_host {
        let final_host = parsed.host_str().unwrap_or("");
        if final_host.eq_ignore_ascii_case(target_host) {
            return Some(BlockReason::LoginWall);
        }
    } else {
        return Some(BlockReason::LoginWall);
    }
    None
}

/// Build the captured evidence payload (truncated HTML + URL).
fn build_evidence(html: &str, final_url: Option<&str>) -> BlockEvidence {
    const MAX_SNIPPET: usize = 4096;
    let snippet = if html.len() > MAX_SNIPPET {
        // Truncate at a char boundary to avoid splitting a UTF-8 sequence.
        let mut cut = MAX_SNIPPET;
        while cut > 0 && !html.is_char_boundary(cut) {
            cut -= 1;
        }
        Some(html[..cut].to_string())
    } else {
        Some(html.to_string())
    };
    BlockEvidence {
        final_url: final_url.map(String::from),
        html_snippet: snippet,
        status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflare_turnstile_iframe_detected() {
        let html =
            r#"<iframe src="https://challenges.cloudflare.com/turnstile/v0/widget"></iframe>"#;
        match detect_captcha(html) {
            Some(BlockReason::Captcha { provider }) => {
                assert_eq!(provider, "cloudflare-turnstile");
            }
            other => panic!("expected cloudflare-turnstile, got {other:?}"),
        }
    }

    #[test]
    fn hcaptcha_detected() {
        let html = r#"<script src="https://js.hcaptcha.com/1/api.js"></script>"#;
        match detect_captcha(html) {
            Some(BlockReason::Captcha { provider }) => assert_eq!(provider, "hcaptcha"),
            other => panic!("expected hcaptcha, got {other:?}"),
        }
    }

    #[test]
    fn recaptcha_detected() {
        let html = r#"<input type="hidden" name="g-recaptcha-response">"#;
        match detect_captcha(html) {
            Some(BlockReason::Captcha { provider }) => assert_eq!(provider, "recaptcha"),
            other => panic!("expected recaptcha, got {other:?}"),
        }
    }

    #[test]
    fn vanilla_page_not_classified_as_captcha() {
        let html = "<html><body><h1>Welcome</h1><p>Just a normal page.</p></body></html>";
        assert!(detect_captcha(html).is_none());
    }

    #[test]
    fn cloudflare_hosted_site_does_not_trigger_challenge_detection() {
        // Pages merely served via Cloudflare often include `cf-mitigated`-style
        // markers in headers but not in body. A single body marker should not
        // fire the challenge-page detector.
        let html = "<html><body>cf-mitigated: a-real-page</body></html>";
        assert!(detect_cloudflare_challenge(html).is_none());
    }

    #[test]
    fn cloudflare_interstitial_with_two_markers_detected() {
        let html = r#"<html><head><title>Just a moment...</title></head>
            <body>
            <h1>Checking your browser before accessing</h1>
            <script src="/cdn-cgi/challenge-platform/x/y/z.js"></script>
            </body></html>"#;
        match detect_cloudflare_challenge(html) {
            Some(BlockReason::FraudDetect { vendor }) => assert_eq!(vendor, "cloudflare"),
            other => panic!("expected cloudflare fraud-detect, got {other:?}"),
        }
    }

    #[test]
    fn login_path_with_matching_host_detected() {
        let reason = detect_login_wall(
            Some("https://example.com/login?return_to=/dashboard"),
            Some("example.com"),
        );
        assert!(matches!(reason, Some(BlockReason::LoginWall)));
    }

    #[test]
    fn login_path_with_no_target_host_still_detected() {
        let reason = detect_login_wall(Some("https://example.com/account/login"), None);
        assert!(matches!(reason, Some(BlockReason::LoginWall)));
    }

    #[test]
    fn login_path_on_different_host_does_not_fire() {
        let reason = detect_login_wall(
            Some("https://identity-provider.com/login"),
            Some("example.com"),
        );
        assert!(reason.is_none());
    }

    #[test]
    fn dashboard_url_not_login_wall() {
        let reason = detect_login_wall(Some("https://example.com/dashboard"), Some("example.com"));
        assert!(reason.is_none());
    }

    #[test]
    fn evidence_truncates_long_html_at_char_boundary() {
        let html = "x".repeat(8192);
        let detection = classify(
            &format!(
                r#"<iframe src="https://challenges.cloudflare.com/turnstile/v0/widget"></iframe>{html}"#
            ),
            Some("https://example.com"),
            None,
        );
        let evidence = detection.expect("captcha should fire").evidence;
        let snippet = evidence.html_snippet.expect("snippet present");
        assert!(snippet.len() <= 4096);
    }
}
