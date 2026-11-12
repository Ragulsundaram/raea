---
title: "Publish"
summary: "Connect the accounts you post to, then post the approved cut to all of them at once from Publish, with each network's fields and checks."
order: 7
sources: ["apps/app/src/publish", "apps/app/src/plugins/catalog.ts", "apps/site/api/src/connect", "docs/connections.feature"]
tutorial: "publish"
---

When a cut is approved, raea posts it for you. This tutorial connects accounts, reads the Publish page, and posts one
cut to several networks at once. Publishing comes with the Pro and Team plans.

## Connect your accounts

Before you publish, connect the accounts you post to. Open Plugins, and under Social, choose a network, such as
YouTube, LinkedIn or Bluesky, then sign in. raea posts only when you press Publish, and it never reads your feed or your
messages.

## Open Publish

Now open Publish in the sidebar. At the top is what's ready to go out: work your agents have marked ready, each with a
caption drafted for its accounts. Below that are your posts, the scheduled ones first, then what's already out, with
each post's views, likes, comments and shares.

## Compose a post

Open a cut that's ready. Choose the accounts it goes to, and write the caption once. Each network has its own fields,
such as a title and privacy on YouTube, and raea only asks for the ones your accounts need.

## Checks

Before anything goes out, raea checks the video against each network's rules: its length, its shape, and its caption's
limit. A check that fails says what's wrong, so you fix it before posting, not after.

## Post it

Press Publish to post now, or pick a time to schedule it. Each account shows its progress as it goes: uploading,
processing, then published. One press posts to every account you chose.

Next: [Extend raea with the SDK](extend.md).
