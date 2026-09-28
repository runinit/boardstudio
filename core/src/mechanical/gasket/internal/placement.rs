//! Plan support groups on usable straight runs, independently of compass-side quotas.
use super::*;

const STRAIGHTNESS_TOLERANCE: f64 = 0.1;

struct Run {
    points: Vec<Vec2>,
    start: f64,
    end: f64,
    span: f64,
}

fn merge(a: &Run, b: &Run) -> Option<Run> {
    let start = a.points[0];
    let end = *b.points.last()?;
    let length = distance(start, end);
    if length < 1e-6 {
        return None;
    }
    let tangent = scale(
        Vec2 {
            x: end.x - start.x,
            y: end.y - start.y,
        },
        1. / length,
    );
    let points: Vec<_> = a
        .points
        .iter()
        .chain(b.points.iter().skip(1))
        .copied()
        .collect();
    // Ignore tiny outline jogs, but never bridge a corner or a reversal.
    if points.iter().any(|p| {
        let x = p.x - start.x;
        let y = p.y - start.y;
        (x * tangent.y - y * tangent.x).abs() > STRAIGHTNESS_TOLERANCE
            || x * tangent.x + y * tangent.y < -1e-6
            || x * tangent.x + y * tangent.y > length + 1e-6
    }) {
        return None;
    }
    Some(Run {
        points,
        start: a.start,
        end: b.end,
        span: a.span + b.span,
    })
}

fn runs(region: &Region) -> Vec<Run> {
    let mut runs: Vec<Run> = vec![];
    for track in &region.tracks {
        let run = Run {
            points: vec![track.start, track.end],
            start: track.start_anchor,
            end: track.end_anchor,
            span: distance(track.start, track.end),
        };
        if let Some(joined) = runs.last().and_then(|last| merge(last, &run)) {
            *runs.last_mut().unwrap() = joined;
        } else {
            runs.push(run);
        }
    }
    if runs.len() > 1 {
        let first = &runs[0];
        let wrapped = Run {
            points: first.points.clone(),
            start: first.start + 1.,
            end: first.end + 1.,
            span: first.span,
        };
        if let Some(joined) = merge(runs.last().unwrap(), &wrapped) {
            runs.remove(0);
            *runs.last_mut().unwrap() = joined;
        }
    }
    runs
}

pub(super) fn tracks(ring: &Contour, id: &str) -> Vec<MechanicalGasketTrack> {
    let perimeter: f64 = ring
        .points
        .iter()
        .enumerate()
        .map(|(i, a)| distance(*a, ring.points[(i + 1) % ring.points.len()]))
        .sum();
    let mut travelled = 0.;
    ring.points
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = ring.points[(i + 1) % ring.points.len()];
            let start_anchor = travelled / perimeter;
            travelled += distance(*a, b);
            MechanicalGasketTrack {
                region_id: id.into(),
                start: *a,
                end: b,
                start_anchor,
                end_anchor: travelled / perimeter,
            }
        })
        .collect()
}

pub(super) fn closure(
    ring: &Contour,
    available: &[Candidate],
    margin: f64,
    target: f64,
    end_search_radius: f64,
) -> Option<Candidate> {
    let region = Region {
        id: String::new(),
        key: String::new(),
        candidates: vec![],
        tracks: tracks(ring, ""),
    };
    let runs = runs(&region);
    let perimeter: f64 = runs.iter().map(|run| run.span).sum();
    available
        .iter()
        .filter_map(|c| {
            runs.iter().find_map(|run| {
                let anchor = if c.anchor < run.start {
                    c.anchor + 1.
                } else {
                    c.anchor
                };
                let along = (anchor - run.start) / (run.end - run.start) * run.span;
                // Prefer nearby run ends, but never pull a target across a long
                // side merely to keep that run free for pads.
                let delta = (c.anchor - target).abs();
                let target_distance = delta.min(1. - delta) * perimeter;
                (run.span >= 30.
                    && target_distance <= end_search_radius
                    && along >= 0.
                    && along <= run.span
                    && along.min(run.span - along) <= margin + 1.)
                    .then_some((c, target_distance + run.span / 2.))
            })
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c.clone())
        .or_else(|| nearest(available, target))
}

fn side(candidate: &Candidate) -> usize {
    if candidate.normal.x.abs() > candidate.normal.y.abs() {
        if candidate.normal.x > 0. {
            0
        } else {
            1
        }
    } else if candidate.normal.y > 0. {
        2
    } else {
        3
    }
}

fn group(
    region: &Region,
    run: &Run,
    count: usize,
    lengths: &[f64],
    margin: f64,
    occupied: &[(Candidate, f64)],
    fits: &impl Fn(&Candidate, f64, &[(Candidate, f64)]) -> bool,
) -> Option<Vec<(Candidate, f64)>> {
    let cell = run.span / count as f64;
    let maximum_shift = ((cell - lengths.last()?) / 2. - margin).max(0.).floor() as usize;
    let offsets: Vec<_> = std::iter::once(0.)
        .chain((1..=maximum_shift).flat_map(|step| [step as f64, -(step as f64)]))
        .collect();
    let mut options: Vec<_> = lengths
        .iter()
        .flat_map(|length| offsets.iter().map(move |offset| (*length, *offset)))
        .collect();
    options.sort_by(|(la, oa), (lb, ob)| {
        if count > 1 {
            oa.abs().total_cmp(&ob.abs()).then_with(|| lb.total_cmp(la))
        } else {
            (lb - 2. * ob.abs()).total_cmp(&(la - 2. * oa.abs()))
        }
    });
    for (length, offset) in options {
        let slack = (cell - length) / 2. - margin;
        if slack < offset.abs() {
            continue;
        }
        let mut pads = vec![];
        for index in 0..count {
            let fraction = ((index as f64 + 0.5) * cell + offset) / run.span;
            let candidate = on_track(
                region,
                (run.start + (run.end - run.start) * fraction).rem_euclid(1.),
            )?;
            let all: Vec<_> = occupied.iter().chain(pads.iter()).cloned().collect();
            if !fits(&candidate, length, &all) {
                break;
            }
            pads.push((candidate, length));
        }
        if pads.len() == count {
            return Some(pads);
        }
    }
    None
}

pub(super) fn plan(
    region: &Region,
    budget: Option<usize>,
    lengths: &[f64],
    margin: f64,
    pinned: &[Candidate],
    fits: impl Fn(&Candidate, f64, &[(Candidate, f64)]) -> bool,
) -> Vec<(Candidate, f64)> {
    let limit = budget.unwrap_or(MAX_SUPPORTS.saturating_sub(pinned.len()));
    let mut remaining = runs(region);
    // A pinned support owns its run. It must not be silently moved or crowded.
    remaining.retain(|run| {
        !pinned.iter().any(|p| {
            let anchor = if p.anchor < run.start {
                p.anchor + 1.
            } else {
                p.anchor
            };
            anchor >= run.start && anchor <= run.end
        })
    });
    let mut selected: Vec<(Run, Vec<(Candidate, f64)>)> = vec![];
    let mut covered: Vec<_> = pinned.iter().map(side).collect();
    while !remaining.is_empty() {
        remaining.sort_by(|a, b| {
            let covered_run = |run: &Run| {
                on_track(region, ((run.start + run.end) / 2.).rem_euclid(1.))
                    .is_some_and(|c| covered.contains(&side(&c)))
            };
            covered_run(a)
                .cmp(&covered_run(b))
                .then_with(|| b.span.total_cmp(&a.span))
        });
        let run = remaining.remove(0);
        let occupied: Vec<_> = selected
            .iter()
            .flat_map(|(_, pads)| pads.iter().cloned())
            .collect();
        let available = limit.saturating_sub(occupied.len());
        if available == 0 {
            break;
        }
        let desired = if budget.is_some() {
            1
        } else {
            ((run.span - 2. * margin) / lengths[0]).ceil().max(1.) as usize
        };
        for count in (1..=desired.min(available)).rev() {
            if let Some(pads) = group(region, &run, count, lengths, margin, &occupied, &fits) {
                covered.extend(pads.iter().map(|(c, _)| side(c)));
                selected.push((run, pads));
                break;
            }
        }
    }
    // An explicit count subdivides the least-supported runs instead of filling a side quota.
    while budget.is_some() && selected.iter().map(|(_, pads)| pads.len()).sum::<usize>() < limit {
        let mut order: Vec<_> = (0..selected.len()).collect();
        order.sort_by(|&a, &b| {
            (selected[b].0.span / (selected[b].1.len() + 1) as f64)
                .total_cmp(&(selected[a].0.span / (selected[a].1.len() + 1) as f64))
        });
        let mut added = false;
        for index in order {
            let occupied: Vec<_> = selected
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .flat_map(|(_, (_, pads))| pads.iter().cloned())
                .collect();
            let (run, pads) = &selected[index];
            if let Some(next) = group(
                region,
                run,
                pads.len() + 1,
                lengths,
                margin,
                &occupied,
                &fits,
            ) {
                selected[index].1 = next;
                added = true;
                break;
            }
        }
        if !added {
            break;
        }
    }
    selected.into_iter().flat_map(|(_, pads)| pads).collect()
}
