/// Initial or queued navigation target for a [`WebTab`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadTarget {
    Url(String),
    Html(String),
}

impl LoadTarget {
    fn is_about_blank(&self) -> bool {
        matches!(self, Self::Url(u) if u.eq_ignore_ascii_case("about:blank"))
    }

    fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }
}

/// Pure readiness state machine coordinating initial loads with `servo-gtk`'s
/// asynchronous browsing-context handshake (`about:blank` `LoadEvent::Finished`).
///
/// Prevents two startup race bugs:
/// 1. Dropping initial loads when a fallback timer fires before Servo is ready.
/// 2. Overwriting a tab's real URL with Servo's transient `about:blank` or
///    `load_html`'s generated `data:text/html` URI.
#[derive(Debug)]
pub struct LoadCoordinator {
    handshake_done: bool,
    pending: Option<LoadTarget>,
    fallback_dispatched: Option<LoadTarget>,
    showing_builtin_html: bool,
}

impl LoadCoordinator {
    pub fn new(initial: LoadTarget) -> Self {
        let showing_builtin_html = initial.is_html();
        Self {
            handshake_done: false,
            pending: Some(initial),
            fallback_dispatched: None,
            showing_builtin_html,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.handshake_done
    }

    /// Queue a load if the browsing context is not ready yet, or return it
    /// immediately for dispatch once ready.
    pub fn request_load(&mut self, target: LoadTarget) -> Option<LoadTarget> {
        self.showing_builtin_html = target.is_html();
        if self.handshake_done {
            self.fallback_dispatched = None;
            Some(target)
        } else {
            self.pending = Some(target);
            self.fallback_dispatched = None;
            None
        }
    }

    /// Invoked if the fallback timer fires before Servo finishes its initial
    /// `about:blank` handshake. Dispatches the queued load while keeping a
    /// retry copy in case Servo drops it for an unknown browsing context.
    pub fn on_fallback_timeout(&mut self) -> Option<LoadTarget> {
        if self.handshake_done {
            return None;
        }
        let target = self.pending.take()?;
        self.fallback_dispatched = Some(target.clone());
        Some(target)
    }

    /// Invoked when Servo emits `LoadEvent::Finished` with `current_uri`.
    ///
    /// Returns `Some(target)` when an initial or dropped fallback load should
    /// now be dispatched to Servo.
    pub fn on_load_finished(&mut self, current_uri: Option<&str>) -> Option<LoadTarget> {
        let is_blank = current_uri.is_none_or(|u| u.is_empty() || u == "about:blank");

        if !self.handshake_done {
            self.handshake_done = true;
            if let Some(target) = self.pending.take() {
                self.fallback_dispatched = None;
                return Some(target);
            }
            if is_blank
                && let Some(target) = self.fallback_dispatched.take()
                && !target.is_about_blank()
            {
                return Some(target);
            }
            self.fallback_dispatched = None;
            return None;
        }

        if is_blank
            && let Some(target) = self.fallback_dispatched.take()
            && !target.is_about_blank()
        {
            return Some(target);
        }

        self.fallback_dispatched = None;
        None
    }

    /// Filter Servo `notify::uri` updates. Returns `false` for transient
    /// startup `about:blank` notifications or generated `data:text/html` URIs
    /// belonging to built-in HTML pages (`about:help`).
    pub fn accept_uri_change(&mut self, uri: &str) -> bool {
        if uri == "about:blank" {
            let awaiting_non_blank = self
                .pending
                .as_ref()
                .or(self.fallback_dispatched.as_ref())
                .is_some_and(|t| !t.is_about_blank());
            if !self.handshake_done || awaiting_non_blank {
                return false;
            }
        }

        if self.showing_builtin_html && uri.starts_with("data:text/html") {
            self.handshake_done = true;
            self.fallback_dispatched = None;
            return false;
        }

        if uri != "about:blank" {
            self.handshake_done = true;
            self.fallback_dispatched = None;
            self.showing_builtin_html = false;
        }
        true
    }
}
