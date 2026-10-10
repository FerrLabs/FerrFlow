---
title: 'FerrFlow: Tags are pushed before releases are published'
summary: 'A release run now pushes its tags to the remote before creating the GitHub or Gitea release, so a failure in between can no longer leave a published release pointing at a tag that never landed: and annotated tag messages are no longer silently replaced.'
date: 2026-07-27T10:00:00Z
product: ferrflow
type: fix
prLink: https://github.com/FerrLabs/FerrFlow/pull/771
---

A release run used to create the forge releases first and push the annotated tags afterwards. Any failure in between: a network blip, branch protection, a tag a concurrent run had just taken: left the forge ahead of git: releases published against a ref the remote did not have.

On GitHub and Gitea it also cost you the tag message. FerrFlow sent the release with a target commit, so for a tag that did not exist yet those APIs created it themselves, as a *lightweight* tag. The annotated tag FerrFlow had built: the one carrying the changelog body: was pushed a moment later, pointed at the same commit, and was therefore treated as already synced and skipped. The lightweight tag was what survived.

Tags now land first, and the release is created against a ref that provably exists. FerrFlow no longer sends a target commit at all, so the forge can't invent tags behind git's back, and `git show <tag>` returns the release notes as intended. Pushing and publishing are also separate resume points now: if release creation fails, re-running retries only that step instead of replaying the push.
