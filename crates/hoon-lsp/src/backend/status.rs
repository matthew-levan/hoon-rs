use super::*;

impl Backend {
    pub(super) fn notify_status(&self, level: MessageType, message: impl Into<String>) {
        let client = self.client.clone();
        let message = message.into();
        tokio::spawn(async move {
            client.log_message(level, message).await;
        });
    }

    pub(super) fn notify_user(&self, level: MessageType, message: impl Into<String>) {
        let client = self.client.clone();
        let message = message.into();
        tokio::spawn(async move {
            client.show_message(level, message).await;
        });
    }

    pub(super) fn notify_status_throttled(
        &self,
        key: &str,
        level: MessageType,
        message: impl Into<String>,
        cooldown: Duration,
    ) {
        let now = Instant::now();
        if let Some(mut last) = self.status_throttle.get_mut(key) {
            if now.duration_since(*last) < cooldown {
                return;
            }
            *last = now;
        } else {
            self.status_throttle.insert(key.to_string(), now);
        }
        self.notify_status(level, message);
    }
}
