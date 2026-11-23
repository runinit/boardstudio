use crate::model::*;

/// Queries that resolve a stored overlay to its mirrored-half correspondence.
impl MirroredHalves {
    /// The correspondence an exception overlays: same kind and mirror item. An exception without
    /// one is a legacy, unassociated record.
    pub fn overlaid_correspondence(
        &self,
        exception: &MirroredException,
    ) -> Option<&MirroredCorrespondence> {
        self.correspondences.iter().find(|correspondence| {
            correspondence.kind == exception.kind && correspondence.mirror_id == exception.mirror_id
        })
    }

    /// The authored item an overlay belongs to, from its correspondence.
    pub fn overlay_source(&self, exception: &MirroredException) -> Option<&str> {
        self.overlaid_correspondence(exception)
            .map(|correspondence| correspondence.source_id.as_str())
    }

    /// The state overlay on a correspondence, if the mirror item is Local or Omitted.
    pub fn overlay_of(
        &self,
        correspondence: &MirroredCorrespondence,
    ) -> Option<&MirroredException> {
        self.exceptions.iter().find(|exception| {
            exception.kind == correspondence.kind && exception.mirror_id == correspondence.mirror_id
        })
    }
}
