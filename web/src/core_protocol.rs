use boardstudio_core::model::CoreRequest;

pub fn encode_core_request(request: &CoreRequest) -> Result<String, serde_json::Error> {
    serde_json::to_string(request)
}

#[cfg(test)]
mod tests {
    use super::encode_core_request;
    use boardstudio_core::model::{CoreRequest, EditCommand, EditOperation, EditPhase};

    #[test]
    fn text_frame_preserves_full_u64_revision_without_a_js_number_round_trip() {
        let request = CoreRequest::Edit {
            id: "op-wide".into(),
            command: EditCommand {
                base_revision: u64::MAX,
                transaction_id: "gesture".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::MoveParts { positions: vec![] },
            },
        };
        let frame = encode_core_request(&request).expect("request serializes");
        assert!(frame.contains("\"baseRevision\":18446744073709551615"));
        let decoded: CoreRequest = serde_json::from_str(&frame).expect("request stays valid JSON");
        assert_eq!(decoded, request);
    }
}
