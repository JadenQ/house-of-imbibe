# Issue Tracker

This repo uses **local markdown** as its issue tracker. The engineering skills
(`to-prd`, `to-issues`, `triage`, `qa`) read from and write to it.

- **Location**: `.scratch/issues/` — one file per issue, named `NNNN-kebab-slug.md`.
- **Format**: each file has YAML frontmatter (`id`, `title`, `status`, `labels`,
  `feature`, `blocked_by`) followed by a markdown body.
- **PRs as a request surface**: not applicable (local markdown, no PR surface).

## Triage label vocabulary

The five canonical roles, represented as values in the `labels` frontmatter list:

| Label | Meaning |
|---|---|
| `needs-triage` | Maintainer needs to evaluate |
| `needs-info` | Waiting on reporter for more information |
| `ready-for-agent` | Ready for an AFK agent to pick up |
| `ready-for-human` | Ready for a human to pick up |
| `wontfix` | Will not be fixed |

New issues created by `to-prd` / `to-issues` are published with `ready-for-agent`.

## Conventions

- Issue `id` is a zero-padded integer matching the filename prefix (`0001`, `0002`, …).
- `blocked_by` is a list of issue `id`s (integers) that must complete first.
- The parent PRD issue links to its child issues via a `## Child issues` section;
  children link back via `## Parent`.
- Status values: `ready-for-agent` | `ready-for-human` | `needs-info` | `in-progress` | `done` | `wontfix`.
