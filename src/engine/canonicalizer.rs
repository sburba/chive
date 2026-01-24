use crate::engine::hex::{Hex, RotationDegrees};
use rustc_hash::FxHashMap;
use std::cmp::{Ordering, min};
use strum::IntoEnumIterator;

fn canonicalize_translation<T>(hexes: &mut Vec<(Hex, T)>) {
    let mut min_q = i32::MAX;
    let mut min_r = i32::MAX;

    for (hex, _) in hexes.iter() {
        min_q = min(min_q, hex.q);
        min_r = min(min_r, hex.r);
    }

    for (hex, _) in hexes {
        hex.q -= min_q;
        hex.r -= min_r;
        // h intentionally untouched
    }
}

pub fn canonicalize<T: Clone + Ord>(map: &FxHashMap<Hex, T>) -> FxHashMap<Hex, T> {
    let mut best: Option<Vec<(Hex, &T)>> = None;

    for rotation in RotationDegrees::iter() {
        let mut rotated: Vec<(Hex, &T)> = map
            .iter()
            .map(|(hex, val)| (hex.rotated_by(rotation), val))
            .collect();

        canonicalize_translation(&mut rotated);

        rotated.sort();

        // Pick lexicographically minimal
        best = match best {
            None => Some(rotated),
            Some(value) => {
                if value.cmp(&rotated) == Ordering::Less {
                    Some(rotated)
                } else {
                    Some(value)
                }
            }
        };
    }

    // Rebuild map
    let mut result = FxHashMap::default();
    for (hex, value) in best.unwrap() {
        result.insert(hex, value.clone());
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::engine::hex::Hex;
    use proptest::prelude::*;
    use rustc_hash::FxHashMap;

    fn hex_strategy() -> impl Strategy<Value=Hex> {
        (-5..=5, -5..=5, 0..=2).prop_map(|(q, r, h)| Hex { q, r, h })
    }

    fn hex_map_strategy() -> impl Strategy<Value=FxHashMap<Hex, String>> {
        prop::collection::hash_map(hex_strategy(), r"[a-zA-Z]", 1..=42)
            .prop_map(|map| map.into_iter().collect())
    }

    fn rotation_strategy() -> impl Strategy<Value=RotationDegrees> {
        proptest::sample::select(RotationDegrees::iter().collect::<Vec<_>>())
    }

    proptest! {
        #[test]
        fn translations_and_rotations_do_not_affect_canonical_form(
            original_map in hex_map_strategy(),
            q_translation in -5..5,
            r_translation in -5..5,
            rotation in rotation_strategy(),
        ) {
            let translated_map: FxHashMap<Hex, String> = original_map
                .iter()
                .map(|(hex, val)| {
                    (
                        Hex {
                            q: hex.q + q_translation,
                            r: hex.r + r_translation,
                            h: hex.h,
                        }
                        .rotated_by(rotation),
                        val.clone(),
                    )
                })
                .collect();

            assert_eq!(canonicalize(&original_map), canonicalize(&translated_map))
        }
    }
}
