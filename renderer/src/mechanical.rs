#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StackLayerInput {
    id: String,
    z: f32,
    thickness: f32,
}

fn stack_index(stack: &[StackLayerInput], id: &str) -> i32 {
    let Some(layer) = stack.iter().find(|layer| layer.id == id) else {
        return -1;
    };
    let center = layer.z + layer.thickness / 2.;
    let mut heights = stack
        .iter()
        .map(|layer| layer.z + layer.thickness / 2.)
        .filter(|height| *height > center + 0.001)
        .collect::<Vec<_>>();
    heights.sort_by(|a, b| b.total_cmp(a));
    heights.dedup_by(|a, b| (*a - *b).abs() < 0.001);
    heights.len() as i32
}

fn explode_offset(index: i32) -> f32 {
    if index <= 0 {
        0.0
    } else {
        -(index as f32) * 2.4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exploded_stack_increases_adjacent_clearances() {
        let stack = [
            ("plate", 5.0, 1.5),
            ("plate-foam", 0.0, 3.0),
            ("pcb", -1.6, 1.6),
            ("bottom-foam", -3.6, 2.0),
            ("battery", -9.6, 6.0),
            ("bottom", -12.6, 3.0),
        ]
        .map(|(id, z, thickness)| StackLayerInput {
            id: id.into(),
            z,
            thickness,
        });

        let offsets: Vec<_> = stack
            .iter()
            .map(|layer| explode_offset(stack_index(&stack, &layer.id)))
            .collect();
        for index in 0..stack.len() - 1 {
            let assembled = stack[index].z - (stack[index + 1].z + stack[index + 1].thickness);
            let exploded = stack[index].z + offsets[index]
                - (stack[index + 1].z + offsets[index + 1] + stack[index + 1].thickness);
            assert!(exploded > assembled);
            assert!(exploded > 0.0);
        }
        for (offset, expected) in offsets.iter().zip([0.0, -2.4, -4.8, -7.2, -9.6, -12.0]) {
            assert!((offset - expected).abs() < 1e-5);
        }
        assert_eq!(explode_offset(-1), 0.0);
    }

    #[test]
    fn stack_index_ranks_by_height_and_handles_ties_and_unknown_layers() {
        let stack = [
            ("lower", -5.0, 2.0),
            ("same", 1.0, 2.0),
            ("target", -1.0, 2.0),
            ("upper", 5.0, 2.0),
            ("same-height", 1.0, 2.0),
        ]
        .map(|(id, z, thickness)| StackLayerInput {
            id: id.into(),
            z,
            thickness,
        });

        assert_eq!(stack_index(&stack, "upper"), 0);
        assert_eq!(stack_index(&stack, "same"), 1);
        assert_eq!(stack_index(&stack, "same-height"), 1);
        assert_eq!(stack_index(&stack, "target"), 2);
        assert_eq!(stack_index(&stack, "lower"), 3);
        assert_eq!(stack_index(&stack, "missing"), -1);
    }
}
