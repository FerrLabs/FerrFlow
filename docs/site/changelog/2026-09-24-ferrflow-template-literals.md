---
title: 'FerrFlow: versionTemplate now tells you about a bad literal instead of git'
summary: 'A versionTemplate whose literal text carried a space, a tab or a character git reserves produced a version that could not become a tag, and the run died at git tag with a message about ref names. The template is now checked where it is read.'
date: 2026-09-24T13:40:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/1194
docsLink: https://ferrflow.com/schema/ferrflow.json
---

`versionTemplate` checked that every `{variable}` was one it knows, and let the literal text between them through untouched. So a template with a stray space or tab produced a version carrying it, and that version went on to be a git tag, which cannot hold either.

The release failed, but at the end, with git's complaint about the ref name rather than ours about the template. You were told the tag was bad, which is true and useless.

The literals are now checked when the template is read, by `ferrflow check` and by the release alike. They may not carry whitespace, control characters, or the characters git reserves in a ref name (`~ ^ : ? * [ \`), and the error names the one it found:

```
versionTemplate "{major}.{minor}\t" has a control character ('\t') in a literal. The version becomes a git tag, which cannot carry it.
```

Nothing that worked before stops working. A template rejected today already failed, later and less clearly. If `ferrflow check` starts complaining after this upgrade, it is telling you about a release that was going to break.
