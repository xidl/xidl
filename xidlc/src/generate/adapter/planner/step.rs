//! Build one adjacent plan and derive referenced type conversions.

use super::*;

pub(super) struct StepPlanner<'a> {
    pub(super) source: &'a VersionSurface,
    pub(super) target: &'a VersionSurface,
    pub(super) facts: &'a MapStep,
    pub(super) contract: &'a str,
    pub(super) versions: (&'a MapVersion, &'a MapVersion),
}

impl StepPlanner<'_> {
    pub(super) fn plan(&self) -> IdlcResult<(Vec<TypePlan>, Vec<OperationPlan>)> {
        self.validate_presence()?;
        let paired = self
            .source
            .types
            .keys()
            .filter(|key| self.target.types.contains_key(*key))
            .cloned()
            .collect::<BTreeSet<_>>();
        let up = ReferencePlanner {
            surface: self.source,
            paired: &paired,
            version: &self.facts.from,
            context: self.contract,
        };
        let down = ReferencePlanner {
            surface: self.target,
            paired: &paired,
            version: &self.facts.to,
            context: self.contract,
        };
        let operations = self.operations(&up, &down)?;
        let mut upward = BTreeMap::new();
        let mut downward = BTreeMap::new();
        let mut up_roots = Vec::new();
        let mut down_roots = Vec::new();
        for operation in &operations {
            up_roots.extend(
                operation
                    .upcast
                    .copies
                    .iter()
                    .map(|copy| copy.value.clone()),
            );
            down_roots.extend(
                operation
                    .downcast
                    .copies
                    .iter()
                    .map(|copy| copy.value.clone()),
            );
            down_roots.extend(
                operation
                    .source
                    .raises
                    .iter()
                    .filter(|exception| operation.target.raises.contains(exception))
                    .cloned()
                    .map(ValuePlan::Named),
            );
        }
        // Only traverse requested directions; hooked subtrees do not require adapters.
        self.derive_references(&up_roots, Direction::Upcast, &up, &mut upward)?;
        self.derive_references(&down_roots, Direction::Downcast, &down, &mut downward)?;
        let keys = upward
            .keys()
            .chain(downward.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let types = keys
            .into_iter()
            .map(|key| TypePlan {
                upcast: upward.remove(&key),
                downcast: downward.remove(&key),
                key,
            })
            .collect();
        Ok((types, operations))
    }

    /// Derive reference effects here so this policy can later move into core.
    fn derive_references(
        &self,
        roots: &[ValuePlan],
        direction: Direction,
        references: &ReferencePlanner<'_>,
        conversions: &mut BTreeMap<String, TypeConversion>,
    ) -> IdlcResult<()> {
        loop {
            let reachable = ReferencePlanner::reachable(conversions, roots);
            let pending = reachable
                .into_iter()
                .filter(|key| !conversions.contains_key(key))
                .collect::<Vec<_>>();
            if pending.is_empty() {
                break;
            }
            for key in pending {
                let conversion = self.type_conversion(&key, direction, references)?;
                conversions.insert(key, conversion);
            }
        }
        Ok(())
    }

    fn type_conversion(
        &self,
        key: &str,
        direction: Direction,
        references: &ReferencePlanner<'_>,
    ) -> IdlcResult<TypeConversion> {
        let old = self
            .source
            .types
            .get(key)
            .ok_or_else(|| self.error(key, "missing source type"))?;
        let new = self
            .target
            .types
            .get(key)
            .ok_or_else(|| self.error(key, "missing target type"))?;
        let facts = self.facts.types.get(key);
        let empty = BTreeMap::new();
        let changes = facts.map(|fact| &fact.members).unwrap_or(&empty);
        match (old, new) {
            (TypeShape::Struct(old), TypeShape::Struct(new))
            | (TypeShape::Enum(old), TypeShape::Enum(new))
            | (TypeShape::Union(old), TypeShape::Union(new)) => {
                MapFacts::validate_members(changes, old, new, key)?;
                let kind = match self.source.types.get(key) {
                    Some(TypeShape::Enum(_) | TypeShape::Union(_)) => MemberSetKind::Variants,
                    _ => MemberSetKind::Fields,
                };
                let (source, target) = if direction == Direction::Upcast {
                    (old, new)
                } else {
                    (new, old)
                };
                let mut plan = MemberClassifier {
                    source,
                    target,
                    facts: changes,
                    direction,
                    kind,
                    context: key,
                }
                .plan()?;
                references.copies(&mut plan)?;
                Ok(match self.source.types.get(key) {
                    Some(TypeShape::Enum(_)) => TypeConversion::Enum(plan),
                    Some(TypeShape::Union(_)) => TypeConversion::Union(plan),
                    _ => TypeConversion::Struct(plan),
                })
            }
            (TypeShape::Alias(old), TypeShape::Alias(new)) => {
                if facts.is_some() {
                    return Ok(TypeConversion::Hook);
                }
                let value = if direction == Direction::Upcast {
                    old
                } else {
                    new
                };
                Ok(TypeConversion::Alias(references.value(value)?))
            }
            (TypeShape::Unsupported(kind), _) | (_, TypeShape::Unsupported(kind))
                if kind == "struct inheritance" =>
            {
                // A flattened base changes the public model layout, so the
                // adapter cannot copy fields mechanically. Keep the decision
                // explicit instead of emitting an incorrect conversion.
                Ok(TypeConversion::Hook)
            }
            (TypeShape::Unsupported(kind), _) | (_, TypeShape::Unsupported(kind)) => {
                Err(self.error(key, &format!("{kind} conversion is not supported")))
            }
            _ => Ok(TypeConversion::Hook),
        }
    }

    fn operations(
        &self,
        up: &ReferencePlanner<'_>,
        down: &ReferencePlanner<'_>,
    ) -> IdlcResult<Vec<OperationPlan>> {
        let mut operations = Vec::new();
        for (key, source) in &self.source.operations {
            let target = self
                .target
                .operations
                .get(key)
                .ok_or_else(|| self.error(key, "cannot serve removed operations"))?;
            if source.http.meta.stream.kind.is_some()
                || target.http.meta.stream.kind.is_some()
                || source.http.meta.upgrade_protocol.is_some()
                || target.http.meta.upgrade_protocol.is_some()
            {
                return Err(self.error(key, "stream and upgrade frame adapters belong to phase 2"));
            }
            let fact = self.facts.operations.get(key);
            let changes = MapFacts::parameters(fact, ParameterSide::Request, key)?;
            MapFacts::validate_members(&changes, &source.inputs, &target.inputs, key)?;
            let mut upcast = MemberClassifier {
                source: &source.inputs,
                target: &target.inputs,
                facts: &changes,
                direction: Direction::Upcast,
                kind: MemberSetKind::Fields,
                context: key,
            }
            .plan()?;
            up.copies(&mut upcast)?;
            let output_facts = self.output_facts(source, target, fact)?;
            let mut downcast = MemberClassifier {
                source: &target.outputs,
                target: &source.outputs,
                facts: &output_facts,
                direction: Direction::Downcast,
                kind: MemberSetKind::Fields,
                context: key,
            }
            .plan()?;
            down.copies(&mut downcast)?;
            operations.push(OperationPlan {
                key: key.clone(),
                source: source.clone(),
                target: target.clone(),
                upcast,
                downcast,
            });
        }
        Ok(operations)
    }

    fn output_facts(
        &self,
        source: &Operation,
        target: &Operation,
        fact: Option<&Change>,
    ) -> IdlcResult<BTreeMap<String, Change>> {
        let result = MapFacts::parameters(fact, ParameterSide::Response, &source.name)?;
        for member in source.outputs.iter().chain(&target.outputs) {
            let old = source.outputs.iter().find(|value| value.key == member.key);
            let new = target.outputs.iter().find(|value| value.key == member.key);
            if matches!((old, new), (Some(old), Some(new)) if MemberClassifier::same_shape(old, new)?)
            {
                continue;
            }
            if !result.contains_key(&member.key) {
                return Err(self.error(
                    &source.name,
                    "version_map omitted an output signature change",
                ));
            }
        }

        Ok(result)
    }

    fn validate_presence(&self) -> IdlcResult<()> {
        for (key, change) in &self.facts.types {
            let valid = match change.change {
                ChangeKind::Added => {
                    !self.source.types.contains_key(key) && self.target.types.contains_key(key)
                }
                ChangeKind::Removed => {
                    self.source.types.contains_key(key) && !self.target.types.contains_key(key)
                }
                ChangeKind::Changed => {
                    self.source.types.contains_key(key)
                        && self.target.types.contains_key(key)
                        && change.from.is_some()
                        && change.to.is_some()
                }
            };
            if !valid {
                return Err(self.error(key, "version_map type presence disagrees with HIR"));
            }
        }
        for (key, change) in &self.facts.operations {
            if change.change == ChangeKind::Removed {
                return Err(self.error(key, "cannot serve removed operations"));
            }
            if change.change == ChangeKind::Changed {
                let from = change
                    .from
                    .as_ref()
                    .and_then(|descriptor| descriptor.get("declaration_id"))
                    .and_then(serde_json::Value::as_str);
                let to = change
                    .to
                    .as_ref()
                    .and_then(|descriptor| descriptor.get("declaration_id"))
                    .and_then(serde_json::Value::as_str);
                if from.is_none() || to.is_none() {
                    return Err(self.error(
                        key,
                        "version_map operation has no source declaration identity",
                    ));
                }
                if from != to {
                    return Err(self.error(key, "cannot serve a replaced operation declaration; operation mapping belongs to phase 2"));
                }
            }
            let valid = match change.change {
                ChangeKind::Added => {
                    !self.source.operations.contains_key(key)
                        && self.target.operations.contains_key(key)
                }
                ChangeKind::Changed => {
                    self.source.operations.contains_key(key)
                        && self.target.operations.contains_key(key)
                        && change.from.is_some()
                        && change.to.is_some()
                }
                ChangeKind::Removed => false,
            };
            if !valid {
                return Err(self.error(
                    key,
                    "version_map operation presence disagrees with HTTP HIR",
                ));
            }
        }
        Ok(())
    }

    fn error(&self, item: &str, reason: &str) -> IdlcError {
        IdlcError::rpc(format!(
            "version adapters for {} {} -> {} at {item}: {reason}",
            self.contract, self.versions.0.value, self.versions.1.value
        ))
    }
}
