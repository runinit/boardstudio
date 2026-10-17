use boardstudio_contracts::KeycapSpec;

fn validate(spec: &KeycapSpec) -> Result<(), String> {
    let numbers = [
        spec.size.x,
        spec.size.y,
        spec.top_size.x,
        spec.top_size.y,
        spec.height,
        spec.tilt,
        spec.dish_depth,
        spec.wall_thickness,
        spec.pose.at.x,
        spec.pose.at.y,
        spec.pose.rotation,
        spec.z,
        spec.travel,
    ];
    if numbers.iter().any(|value| !value.is_finite())
        || !(12.0..=150.0).contains(&spec.size.x)
        || !(12.0..=150.0).contains(&spec.size.y)
        || !(3.0..=20.0).contains(&spec.height)
        || spec.tilt.abs() > 12.0
        || !(0.0..=0.8).contains(&spec.dish_depth)
        || !(0.8..=2.0).contains(&spec.wall_thickness)
        || spec.top_size.x < 5.0
        || spec.top_size.y < 5.0
        || spec.top_size.x > spec.size.x - 2.0
        || spec.top_size.y > spec.size.y - 2.0
        || spec.size.x - 2.0 * spec.wall_thickness <= 0.0
        || spec.size.y - 2.0 * spec.wall_thickness <= 0.0
        || spec.top_size.x - 2.0 * spec.wall_thickness <= 0.0
        || spec.top_size.y - 2.0 * spec.wall_thickness <= 0.0
        || !(1..=5).contains(&spec.row)
        || spec.legend.chars().count() > 12
        || spec.legend.chars().any(char::is_control)
    {
        return Err("Invalid keycap mesh parameters".into());
    }
    Ok(())
}
