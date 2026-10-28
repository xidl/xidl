"""Require each expanded BDD scenario to belong to exactly one CI shard."""

from collections import Counter
from pathlib import Path
import sys

from behave.parser import parse_file
import yaml

root = Path(__file__).resolve().parent.parent
workflow = yaml.safe_load((root / ".github/workflows/check-bddtest.yml").read_text())
entries = workflow["jobs"]["bddtest-shard"]["strategy"]["matrix"]["include"]
shards = {f"bdd_{entry['shard']}" for entry in entries}
counts = Counter()
errors = []
for path in sorted((root / "bdd/features").rglob("*.feature")):
    for scenario in parse_file(str(path)).walk_scenarios():
        tags = {
            str(tag) for tag in scenario.effective_tags if str(tag).startswith("bdd_")
        }
        if len(tags) != 1 or not tags <= shards:
            errors.append(
                f"{scenario.location}: expected one known BDD shard, got {tags}"
            )
        else:
            counts.update(tags)
for shard in sorted(shards):
    if not counts[shard]:
        errors.append(f"{shard}: empty BDD shard")
    print(f"{shard}: {counts[shard]} scenarios")
if errors:
    sys.exit("\n".join(errors))
print(f"All {sum(counts.values())} scenarios belong to exactly one CI shard")
