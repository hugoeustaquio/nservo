/// Default EasyList / ABP compatible filter list seeded on first initialization.
pub const DEFAULT_ADS_LIST: &str = r#"[Adblock Plus 2.0]
! Title: NServo Default Ad & Tracker Filter
! Version: 1.0.0
! Homepage: https://github.com/nservo/nservo

! Generic ad network domains
||doubleclick.net^$third-party
||googlesyndication.com^$third-party
||googleadservices.com^$third-party
||adnxs.com^$third-party
||criteo.com^$third-party
||outbrain.com^$third-party
||taboola.com^$third-party
||popads.net^
||adservice.google.com^

! Trackers and telemetry
||analytics.google.com^
||googletagmanager.com/gtm.js$script
||facebook.net/en_US/fbevents.js$script
||hotjar.com^$script

! Banner keywords and script filters
/ads.js$script
/advertisement.
/banner/ads_
/sponsor_banner_

! Generic element hiding rules (cosmetic filtering)
##.adsbox
##.ad-banner
##.ad-placement
##.ad-wrapper
##.sponsored-post
##iframe[src*="doubleclick.net"]
##iframe[src*="adservice"]
"#;
