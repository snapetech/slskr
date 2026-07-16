use slskr_protocol::{distributed::DistributedMessage, peer::PeerMessage, server::ServerMessage};

pub fn trace_server_message(direction: &str, message: &ServerMessage) {
    tracing::debug!(
        target: "soulseek.client.server",
        direction,
        message_kind = ?std::mem::discriminant(message),
        "server message"
    );
}

pub fn trace_peer_message(username: &str, direction: &str, message: &PeerMessage) {
    tracing::debug!(
        target: "soulseek.client.peer",
        username,
        direction,
        message_kind = ?std::mem::discriminant(message),
        "peer message"
    );
}

pub fn trace_distributed_message(username: &str, direction: &str, message: &DistributedMessage) {
    tracing::debug!(
        target: "soulseek.client.distributed",
        username,
        direction,
        message_kind = ?std::mem::discriminant(message),
        "distributed message"
    );
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use slskr_protocol::server::{LoginRequest, ServerMessage};
    use tracing::{
        field::{Field, Visit},
        span::{Attributes, Id, Record},
        Event, Metadata, Subscriber,
    };

    use super::trace_server_message;

    #[derive(Default)]
    struct RecordingSubscriber {
        fields: Arc<Mutex<String>>,
    }

    impl Subscriber for RecordingSubscriber {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &Event<'_>) {
            event.record(&mut FieldRecorder(&self.fields));
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }

    struct FieldRecorder<'a>(&'a Mutex<String>);

    impl Visit for FieldRecorder<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            use std::fmt::Write as _;

            let mut fields = self.0.lock().expect("recording lock");
            let _ = write!(fields, "{}={value:?};", field.name());
        }
    }

    #[test]
    fn server_tracing_does_not_emit_login_credentials() {
        let fields = Arc::new(Mutex::new(String::new()));
        let subscriber = RecordingSubscriber {
            fields: Arc::clone(&fields),
        };
        let message = ServerMessage::LoginRequest(LoginRequest {
            username: "alice".to_owned(),
            password: "super-secret-password".to_owned(),
            major_version: 160,
            hash: "digest".to_owned(),
            minor_version: 1,
        });

        tracing::subscriber::with_default(subscriber, || {
            trace_server_message("out", &message);
        });

        let fields = fields.lock().expect("recording lock");
        assert!(fields.contains("message_kind="));
        assert!(!fields.contains("alice"));
        assert!(!fields.contains("super-secret-password"));
        assert!(!fields.contains("digest"));
    }
}
