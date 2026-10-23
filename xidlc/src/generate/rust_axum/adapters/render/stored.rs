//! Thread stored values through every conversion that reaches a removed member.

use super::*;

impl AdapterRenderer<'_> {
    /// A conversion threads stored values when its own hook reads them or when
    /// any conversion it calls does; close that set, then mark every call site.
    pub(super) fn thread_stored(&self, file: &mut FileContext) {
        let mut stored = file
            .conversions
            .iter()
            .filter(|conversion| conversion.stored)
            .map(|conversion| conversion.name.clone())
            .collect::<BTreeSet<_>>();
        loop {
            let before = stored.len();
            for conversion in &file.conversions {
                if stored.contains(&conversion.name) {
                    continue;
                }
                let reaches = conversion
                    .copies
                    .iter()
                    .any(|copy| copy.value.references(&stored))
                    || conversion
                        .value
                        .as_ref()
                        .is_some_and(|value| value.references(&stored));
                if reaches {
                    stored.insert(conversion.name.clone());
                }
            }
            if stored.len() == before {
                break;
            }
        }
        for conversion in &mut file.conversions {
            conversion.stored = stored.contains(&conversion.name);
            for copy in &mut conversion.copies {
                copy.value.annotate(&stored);
            }
            if let Some(value) = &mut conversion.value {
                value.annotate(&stored);
            }
        }
        let marked = file
            .conversions
            .iter()
            .map(|conversion| {
                (
                    conversion.name.clone(),
                    (conversion.current, conversion.pair),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for conversion in &mut file.conversions {
            for copy in &mut conversion.copies {
                copy.value.mark_calls(&marked);
            }
            if let Some(value) = &mut conversion.value {
                value.mark_calls(&marked);
            }
        }
        for operation in &mut file.operations {
            for arm in &mut operation.arms {
                if let Some(call) = &mut arm.call {
                    call.stored = stored.contains(&call.name);
                }
            }
            operation.stored = operation
                .arms
                .iter()
                .any(|arm| arm.call.as_ref().is_some_and(|call| call.stored));
        }
        file.needs = self
            .plan
            .needs
            .iter()
            .map(|(key, entries)| Needs {
                key: key.clone(),
                entries: entries.clone(),
            })
            .collect();
    }
}
