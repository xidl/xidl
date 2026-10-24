//! Resolve the update-operation marks for one adjacent step.

use super::*;

/// What one conversion in this step does with the update flow.
#[derive(Clone, Default)]
pub(super) struct UpdateMark {
    pub current: bool,
    pub pair: bool,
    pub stored_type: Option<String>,
    pub paired: Vec<Member>,
    pub head: Option<TsType>,
}

pub(super) struct UpdateMarks<'a> {
    step: &'a StepUpdates,
    updates: &'a Updates,
    head: &'a Names<'a>,
}

impl<'a> UpdateMarks<'a> {
    pub(super) fn new(step: &'a StepUpdates, updates: &'a Updates, head: &'a Names<'a>) -> Self {
        Self {
            step,
            updates,
            head,
        }
    }

    /// Marks for one type conversion of this step.
    pub(super) fn of(&self, key: &str) -> UpdateMark {
        self.mark(key, self.step.current.contains(key))
    }

    /// Marks for the operation envelope, keyed by the operation. The envelope
    /// returns the stored bag whenever its step can produce one, even when it
    /// does not itself receive the Head resource.
    pub(super) fn envelope(&self, key: &str) -> UpdateMark {
        let mut mark = self.mark(key, self.step.current.contains(key));
        if let Some(owner) = self.step.owner.get(key) {
            if self
                .updates
                .operations
                .get(owner)
                .is_some_and(|update| !update.stored.is_empty())
            {
                mark.pair = true;
                mark.stored_type = Some(Names::stored_type(owner));
            }
        }
        mark
    }

    /// The persisted-member bag interface of one update operation.
    pub(super) fn bag(&self, key: &str) -> IdlcResult<Option<StoredBag>> {
        let Some(update) = self.updates.operations.get(key) else {
            return Ok(None);
        };
        let names_keys = self.head.keys.clone();
        let names = Names {
            keys: &names_keys,
            version: self.head.version,
            from: self.head.from,
            upward: false,
        };
        let mut fields = Vec::new();
        for member in &update.stored {
            fields.push(Field {
                key: ts_prop_name(&member.name),
                ty: names.member_type(member)?,
                optional: true,
            });
        }
        Ok(Some(StoredBag {
            name: Names::stored_type(key),
            fields,
        }))
    }

    fn mark(&self, key: &str, current: bool) -> UpdateMark {
        let owner = self.step.owner.get(key);
        let update = owner.and_then(|owner| self.updates.operations.get(owner));
        let pair = current && update.is_some_and(|update| !update.stored.is_empty());
        UpdateMark {
            current,
            pair,
            stored_type: pair.then(|| {
                Names::stored_type(owner.expect("a paired conversion names its operation"))
            }),
            paired: self.step.paired.get(key).cloned().unwrap_or_default(),
            head: update
                .map(|update| self.head.named(&update.return_key))
                .filter(|_| current),
        }
    }
}
