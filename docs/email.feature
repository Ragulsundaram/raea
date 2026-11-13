Feature: Email: sign-in codes, invites, two-factor and notices, sent by raea-api through Resend
  Shane, 11-12: "build resend tooling for raea.io so that for collaborations and invites and other things we have a
  highly sophisticated build out of invite links, forgot passwords, 2fa etc."

  What exists (11-12):
    - Accounts have no passwords. People sign in with GitHub (device and web flow, auth.ts, web.ts), Google (CN9) or
      Apple (CN10), and identities (0003) join them on a verified email onto users (0001: id, email). A web session and
      a computer are both device rows holding a hashed token (the web's cookie lasts 90 days). Email links were
      planned in CN10 and never built.
    - Team invites (account.ts, org_invites 0019) and project invites (collaborators.ts, project_collaborators, R5)
      reach only people who already have a raea account. Anyone else gets 404 no_user ("hasn't signed in to raea
      yet"). Nothing is emailed: the person finds the invite on raea.io/account or in the Library's Shared with you.
    - Review links (review.ts, R4) take guests' comments and approvals, and the owner sees them only in the app.
    - raea-api sends no email. hello@raea.io is a Cloudflare Email Routing forward.
    - Resend: the account has no domains (checked through its API on 11-12). The key Shane pasted on 11-12 has full
      access (it can list API keys).
    - raea.io's mail DNS (11-12): MX is Email Routing (route1-3.mx.cloudflare.net), SPF is
      "v=spf1 include:_spf.mx.cloudflare.net ~all", DMARC is "v=DMARC1; p=reject;". Because of p=reject, every mail
      raea sends must carry a DKIM signature aligned with raea.io, or Gmail and the rest drop it.
    - Resend's terms (11-12): Free sends 100 mails a day and 3,000 a month; Pro is $20 a month for 50,000, with no
      daily limit. The API takes 10 requests a second per team on every plan, shared by all its keys.

  The approach: raea stays passwordless. "Forgot password" becomes "email me a code", which is the same email
  without a password table to protect. Two-factor is passkeys and authenticator apps on top of any way in. Invites go
  to an email address whether or not the person has an account yet. Every mail goes through one module in raea-api
  (src/mail/): templates in git drawn by its own small layout (EM2b), a send log, suppressions fed by Resend's
  webhooks, and a fake Resend for the tests. Shane, later on 11-12: "finish email work and make it fully live ... and
  we have a place to collect emails and build a proper email list, including any checkmarks we need to process to
  allow them to opt in or out at any time" (EM10).

  Rule: EM0. Decisions (Shane, 11-12: "no passwords, and ill upgrade to pro. team only. but anyone who has review
    links like on pro can send emails to review something")
    Scenario: EM0a. No passwords
      Then raea keeps no passwords: a six-digit code or a link by email signs in, a passkey signs in with one touch,
        and there is no "Forgot password?" because there is nothing to forget
      And (why, for the record) passwords would mean a hash table to protect, reset by email, breach checks and
        lockouts, and Workers' WebCrypto caps PBKDF2 at 100,000 iterations (review.ts already hits this) against
        OWASP's 600,000 for SHA-256
    Scenario: EM0b. Resend Pro ($20 a month), done 11-12
      Then the Free plan's 100 a day runs out at about thirty sign-ins plus their invites, and a sign-in code refused
        at quota locks people out, so the account is on Pro before EM3 ships
    Scenario: EM0c. Keys (done 11-12)
      Then the full-access key from 11-12 is kept outside the repository for setup only (domains, webhooks, keys),
        and is rotated once setup is done, since it has been in a chat transcript
      And the Worker's key is sending-only and limited to raea.io; it is raea-api's RESEND_API_KEY. The list needs a
        second, full-access key (RESEND_CONTACTS_KEY), since a sending-only key can't touch contacts
    Scenario: EM0d. Team invites and project invites to people without an account: Team only
      Then Team accounts invite anyone by email; Pro and Free invite only people who already have an account, as today
    Scenario: EM0e. Review links by email: Pro and Team (EM5h)
      Then anyone who can make review links (Pro and Team, R4) can also email one to people to review it
    Scenario: EM0f. Two-factor for teams: optional, with a team setting to require it (EM4d)

  Rule: EM1. Sending from raea.io, set up once
    Scenario: EM1a. raea.io is a Resend domain
      Then raea.io is added in Resend (region us-east-1) and its records go on Cloudflare: DKIM at
        resend._domainkey.raea.io, and the return path on send.raea.io (MX to Amazon SES's feedback host and
        "v=spf1 include:amazonses.com ~all"), exactly as Resend lists them
      And the root's MX, SPF and DMARC stay as they are: Email Routing keeps hello@raea.io, and DKIM (d=raea.io) aligns
        for p=reject
      And DMARC gains rua=mailto:dmarc@raea.io (an Email Routing address), so failing mail shows up in the reports
      And open and click tracking are off for the domain. Click tracking rewrites links, which breaks one-time links
        and puts a third party between people and raea's sign-in; open tracking is a pixel the privacy policy doesn't
        allow
    Scenario: EM1b. One sender
      Then every mail comes from "raea <hello@raea.io>", so a reply reaches a person through Email Routing
      And an invite never carries the inviter's address (review.ts already keeps owners' emails private)
    Scenario: EM1c. Apple's private relay delivers to people who signed in with Apple
      Then raea.io and send.raea.io are registered under Sign in with Apple for Email Communication on team
        BJJZ79J5NL (done 11-12; a domain covers every address at it, hello@raea.io included), and both pass Apple's
        SPF check there: send.raea.io is the envelope sender, and raea.io signs with DKIM
      And without that, every mail to a privaterelay.appleid.com or private.icloud.com address bounces
      And each relay address takes at most 100 mails a day (Apple's limit), which is far past raea's use
    Scenario: EM1d. Secrets
      Then RESEND_API_KEY (sending-only) and RESEND_WEBHOOK_SECRET are Worker secrets, and RESEND_API is a var that the
        tests point at the fake
      And without the key, routes that must send mail answer 503 mail_unavailable and every other route is unaffected

  Rule: EM2. The mail module (raea-api src/mail/)
    Scenario: EM2a. One send()
      Then send(env, {to, kind, data, user?}) renders the kind's template and POSTs to Resend's /emails with an
        Idempotency-Key, so a retried request never sends twice, a tag naming the kind, and a row in mail_log
      And a 429 rate answer is retried after its retry-after, up to three times; a quota 429 or a 5xx is a failure the
        caller sees: sign-in and security mail answer 503 "We couldn't send the email. Try again in a minute", and
        notices wait for the next cron
      And batches (digests, reminders) go out from the hourly cron at under 8 a second, inside Resend's 10 a second
        shared by every key
      And it uses fetch, as the rest of raea-api does, rather than Resend's Node SDK
    Scenario: EM2b. Templates in git
      Then each kind is a function in src/mail/templates.ts that returns a subject and a list of blocks (heading,
        paragraph, button, code, note); src/mail/layout.ts draws the blocks as table-based HTML (one layout that
        matches raea.io: the lockup, black, hairlines; it reads in light and dark mail clients) and as the plain-text
        part, so the two never disagree. Every value is HTML-escaped there
      And not React Email (checked 11-12): its component packages (@react-email/components and each @react-email/*)
        are deprecated, folded into the `react-email` CLI package, which brings Tailwind, Babel, socket.io and esbuild
        with it. One layout of raea's own is smaller than that
      And every link is absolute to raea.io, test/mail-preview.sh writes each kind to an HTML file to look at, and the
        copy is checked by copymatter
      And the kinds: sign_in, email_change, email_changed, invite_team, invite_project, invite_reminder,
        invite_answered, review_request, security_sign_in, security_factor, security_recovery, review_digest,
        review_approved,
        welcome, storage_limit, access_ended, account_deleted
    Scenario: EM2c. A log, suppressions and Resend's webhooks
      Then mail_log keeps (id, user_id, to_sha256, to_domain, kind, resend_id, status, created_at, updated_at): no
        bodies and no address in the clear, enough for support to answer "did my code go out?", pruned after 90 days
      And POST /hooks/resend checks the Svix signature (svix-id, svix-timestamp, svix-signature, five minutes' leeway)
        and moves the status: delivered, delivery_delayed, bounced, complained
      And a hard bounce suppresses the address; a complaint suppresses everything except security mail
      And send() skips suppressed addresses, and raea.io/account shows "We can't reach you at a•••@x.com" with a fix
    Scenario: EM2d. Preferences and one-click unsubscribe
      Then mail falls in categories: security (always on), invites, activity (review comments, people joining) and
        product news (off unless ticked; Resend Broadcasts, later)
      And every mail outside security carries List-Unsubscribe with a one-click POST URL (RFC 8058) and a footer link;
        its token is an HMAC of the address and category, so unsubscribing needs no sign-in
      And someone without an account who gets an invite can choose "Don't send me raea invites" (invite_blocks keeps
        the address's SHA-256)

  Rule: EM3. Signing in by email (CN10's email links), and getting back in
    Scenario: EM3a. One mail holds a code and a link
      When someone types their email on raea.io's sign-in (a fourth way, under GitHub, Google and Apple) or on the
        link page a computer opens (OB-L)
      Then POST /auth/email/start {email, return?} gives the same {sent} answer whether or not the address has an
        account, so nobody can test which addresses are raea's
      And the mail holds a six-digit code and a link, each good once and for 10 minutes; only HMACs of both are stored
      And the code works only in the browser that asked (a short HttpOnly cookie carries the request's nonce), and the
        request dies after five wrong codes; codes compare in constant time
      And opening the link never spends it: mail scanners (Outlook Safe Links, Mimecast, Proofpoint) open every link,
        so the page asks for one press, which POSTs
      And an address gets 5 requests an hour and 20 a day, OPEN_LIMIT throttles by IP, and MAIL_DAILY is a ceiling
        across everyone that guards the bill (as CHECKS_DAILY_USD does)
    Scenario: EM3b. Who it signs in
      Then identityUser(env, "email", address, null, () => address) finds the user with that email (owning the
        mailbox proves it) or makes one unless SIGNUPS_CLOSED, and a computer gets its token through the link code as
        with Google and Apple
      And with two-factor on, the code is the first step only (EM4b)
    Scenario: EM3c. "Can't sign in?" replaces "Forgot password?"
      Then someone who lost their GitHub, Google or Apple account signs in with a code sent to the account's email
      And accounts whose email can't take mail (GitHub's users.noreply.github.com fallback in auth.ts) are asked on
        raea.io/account to add a reachable address, verified by code, before they need it; count them at rollout
    Scenario: EM3d. Changing the account's email
      Then the new address is verified by a code, after step-up (EM4c)
      And the old address is told "Your raea email is now n•••@y.com", with "This wasn't me" good for 7 days, which
        puts the old address back and signs out every device
    Scenario: EM3e. Lost both the mailbox and the second factor
      Then hello@raea.io handles it by hand, from a computer still signed in or the account's Stripe details
      And a recovery takes 7 days, with a mail to the address on file each day, so nobody takes an account quietly

  Rule: EM4. Two-factor and step-up
    Scenario: EM4a. The factors
      Then passkeys (WebAuthn, RP ID raea.io, so raea.io and app.raea.io share them), authenticator apps (TOTP per
        RFC 6238: SHA-1, 30 s, one step either side, a used step never accepted twice, the secret sealed with
        TOKEN_KEY by connect/crypt.ts), and 10 recovery codes shown once and stored hashed
      And no SMS: SIM swaps, cost, and carriers' filtering
      And a passkey can be the whole sign-in ("Sign in with a passkey", a discoverable credential), counting as both
        factors
      And check @simplewebauthn/server in workerd before choosing it; the fallback is verifying ES256 and RS256
        assertions with WebCrypto directly
    Scenario: EM4b. Where the second step is asked
      Then after any first step (GitHub, Google, Apple, an email code), on raea.io and on the link page a computer
        opens; the device row and its token are made only after the second step
      And a token keeps working until it expires or is revoked, so two-factor is asked at sign-in, not per request
    Scenario: EM4c. Step-up for what can't be undone
      Then changing the email, removing a factor or turning two-factor off, new recovery codes, deleting the account,
        making an agent key (agent-keys.ts), and removing a team member ask for a fresh factor (a passkey, TOTP, or an
        email code when two-factor is off) within the last 10 minutes, kept as step_up_at on the device row
    Scenario: EM4d. A team can require two-factor
      Then a team admin's require_2fa gives members without it 7 days with a banner and a mail, after which the team's
        storage (Git LFS, libraries) answers 403 2fa_required until they add one
    Scenario: EM4e. Security mail
      Then a new sign-in (the device's name, city and country from Cloudflare's request.cf, the time, and "Not you?"
        leading to Sign out everywhere), a factor added or removed, a recovery code used and an email changed each
        send a mail
      And security mail ignores preferences; only a hard bounce stops it

  Rule: EM5. Invites by email, with or without an account
    Scenario: EM5a. Invite anyone by email, to a team or a project
      When a team admin (account.ts) or a project's owner on Team (collaborators.ts) types an email with no account
      Then raea-api keeps an email invite (email_invites: id, kind, org_id or owner and repo, email, invited_by,
        token_sha256, created_at, expires_at, reminded_at), counted in TEAM_MEMBERS or COLLABORATORS_MAX like today's,
        and mails it
      And on Accept it becomes an org_invites or project_collaborators row for the person who accepted and is
        answered at once, so the existing accept, headcount and access paths stay as they are
      And someone who already has an account gets the same mail beside the in-app invite, and 404 no_user goes away
    Scenario: EM5b. The invite's mail and page
      Then the mail says "<the inviter's GitHub login or name> invited you to <team or project> on raea", what that
        means in one line (a team: the team's plan covers you; a project: you work in it from your own raea, and its
        storage is the owner's), when it expires, and an Accept button to raea.io/invite/<token>
      And the page shows who and what; signed out, it offers the four ways in with the email filled; then Accept or
        Decline, never on a GET
      And the token is 128 bits, stored as a SHA-256, and good once
    Scenario: EM5c. Accepting with another address
      Then accepting while signed in under another address asks "This invite went to a•••@x.com. Join as b@y.com?",
        and the inviter sees who joined
      And a team admin can lock the team's invites to the exact address
    Scenario: EM5d. Reminders, expiry, taking back, sending again
      Then an invite left unanswered gets one reminder at 3 days and expires at 14, which frees its place
      And the inviter can send it again once a day; taking it back makes the link 404 at once, as review links do
      And the inviter gets a mail when it is accepted or declined (activity)
    Scenario: EM5e. Invite links, with no email
      Then a team admin makes raea.io/join/<token> with a number of uses (at most the free seats), an expiry (at most
        30 days), an optional email domain (only @agency.com addresses join; anyone else asks the admin), and
        "Approve each" on or off; it can be revoked, and every join is mailed to the admins
      And projects have no invite links: a project spends its owner's storage, so each collaborator is named
    Scenario: EM5f. Abuse
      Then only Team accounts invite people without accounts into a team or project (EM0d), at most 20 such invites a
        day within the existing 100, and 5 an hour to one domain; review requests have their own limits (EM5h)
      And an invite carries no free text from the inviter (later: up to 280 characters, plain, no links, from
        accounts older than 7 days)
      And a complaint on an invite stops that account's invites to people without accounts until the admin agent
        (admin.raea.io) or Shane looks
    Scenario: EM5g. Project invites and GitHub
      Then a project's history is on GitHub (R5a), so someone who joined by email without GitHub is asked to connect it
        when they Open in raea, and the owner's app sends GitHub's repo invite then rather than at invite time
    Scenario: EM5h. Send a review link by email (Pro and Team)
      When the owner of a review link (R4), or their agent with the "share" scope, adds email addresses in the Share
        panel or calls POST /v1/review-links/:id/send {to: [emails], note?}
      Then each address gets "<the owner's GitHub login or name> sent you <title> to review", a still of the current
        version, when the link expires, and a Review button to raea.io/r/<token>; a link with a password says so and
        never carries the password
      And each recipient's mail carries their own ?by= tag, so the page fills their name and the owner sees who
        watched, commented and approved; a recipient needs no account
      And a new version on the link (R4d) offers "Tell reviewers" to the same people, never sent on its own
      And the limits: 25 addresses a send, 100 recipients a day for Pro and 300 for Team, a link's sends counted in its
        own row, suppressions and "Don't send me raea review requests" honoured, and a complaint pauses the account's
        sends until the admin agent or Shane looks
      And the note is plain text, up to 280 characters, with links shown as text, from accounts older than 7 days

    Scenario: EM5i. Status (11-12)
      Then built and live: EM5a for teams (email_invites, D1 0045; src/mail/invites.ts), EM5b (the page is
        raea.io/account/?invite=<token>, beside ?signin=), EM5c's "Join as" notice, EM5d (reminder at 3 days, expiry
        at 14, take back, send again once a day, the inviter mailed on accept or decline), EM5f's Team-only rule and
        its limits (20 a day, 5 an hour to one domain), and EM5h's route, POST /v1/review-links/:id/send
      And raea.io's invite page, sign-in banner and the Team page's invites by email are live, and the app's Share
        panel sends a live link by email (EM5h's field, raea-feed PR #4: up to 25 addresses and a note, never an
        agent's to send)
      And not built yet: project invites by email (EM5a's project half, with EM5g), locking a team's invites to the
        exact address (EM5c), invite links (EM5e), pausing invites after a complaint (EM5f), and for EM5h the still,
        per-recipient ?by= tags and "Tell reviewers"
      And tests: test/invites.sh (offline, 11 checks), test/mail-web.sh's invite path in Chrome, and the app's
        host/share.test.ts, src/share/share.test.ts and e2e/share-email.spec.ts

  Rule: EM6. Activity mail (after EM5)
    Scenario: EM6a. Review links as a digest
      Then comments on a person's review links come as one digest per link at most every 15 minutes, and an approval
        comes at once
      And a reviewer without an account can leave an email to hear of new versions and replies; it is confirmed by a
        code first and leaves with one click
    Scenario: EM6b. Reply by email (later)
      Then a digest's Reply-To is a per-thread address (r+<HMAC>@reply.raea.io) received by Resend's inbound on
        reply.raea.io (the root's MX stays with Email Routing), posted as the owner's comment once the sender matches,
        with the quoted text stripped
      And each received mail counts against Resend's monthly quota
    Scenario: EM6c. Account and plan notices
      Then one welcome mail (no drip), storage at 80% and 100% of the allowance, a plan ending that cut off a team's
        or collaborators' access, and the account deleted
      And Stripe and Link send receipts and failed payments; raea doesn't repeat them

  Rule: EM7. Privacy and law
    Scenario: EM7a. The privacy policy names Resend
      Then /privacy/'s processors (legal.ts, beside Cloudflare) add that Resend sends raea's email and sees each
        address and message, and Resend's DPA is accepted in its dashboard. Done 11-12: the processors list and a new
        Email section (/privacy/#mail) saying what's sent, the consent kept for raea news and how to stop each kind
      And an invited person's address is kept only while the invite lives; an opt-out keeps only its SHA-256
      And mail has no tracking pixels and no rewritten links
    Scenario: EM7b. Product news
      Then product news goes only to people who ticked it (EM10), through Resend Broadcasts, with Shinyobjectz LLC's
        postal address in the footer (CAN-SPAM); the D1 raea-waitlist is not mailed news without that consent
      And an issue starts as a Resend draft made by raea-api's news-draft.mts from a short Markdown file: raea's mail
        layout, the raea news segment and topic, from hello@raea.io (replies reach the team through Email Routing),
        Resend's per-recipient unsubscribe link and the postal address in the footer; it is read, tested and sent from
        Resend's dashboard. The first draft waits there (11-12)

  Rule: EM8. Tests, cheapest first
    Scenario: EM8a. Offline against a fake Resend
      Then test/fake-resend.mts (like fake-github.mts) records each POST /emails, can be told to answer 429 or 500,
        and signs webhooks with a test secret
      And test/mail.sh covers: an email sign-in (start, the code read from the fake, five wrong codes end it, a GET of
        the link doesn't spend it, the press signs in), the same answer for an unknown address, two-factor (a TOTP
        code computed in the test, a recovery code, step-up expiring), invites (a person with no account invited,
        signing up through the link, accepted, the headcount, taken back to 404, the 3-day reminder and the expiry on
        a clock override), suppression (a bounce webhook, then a notice skipped and security mail still sent), and
        one-click unsubscribe
      And test/auth.sh, apple.sh, connections.sh and account.sh still pass
    Scenario: EM8b. Live, after EM1 (cents)
      Then production sends to Resend's test addresses delivered@resend.dev, bounced@resend.dev and
        complained@resend.dev, and the webhooks land
      And one real mail each to Gmail, Outlook and an Apple relay address shows DKIM pass and reads in light and dark
    Scenario: EM8c. The pages
      Then the sign-in's email field, /invite/, /join/ and the account's Security tab pass the site's headless page
        checks at desktop and phone width, in both themes

  Rule: EM9. Order of work
    Scenario: EM9a. Steps
      Then 1. EM1's setup. Done 11-12: raea.io added to Resend (domain 961880ce, us-east-1, tracking off), the
        Worker's sending key and a contacts key made, the four DNS records added through Cloudflare's dashboard in
        Shane's browser, and the domain verified; a test mail from hello@raea.io delivered. Resend is on Pro and
        Apple's relay sources are registered (both 11-12)
      And 2. EM2, the module, the fake, the log, suppressions and the webhook, in D1 migration 0044 onward
      And 3. EM3, email sign-in, finishing CN10. It comes first because it gives every account a way back in
      And 4. EM5, invites by email, invite links and review links by email (EM5h): the collaboration ask
      And 5. EM4, two-factor, step-up and security mail
      And 6. EM6 and EM7b; EM6b's reply by email last
      And EM10, the list, went with EM3 (11-12), since the site's sign-up and the account's switches need only EM2
      And each API deploy goes from private HEAD with Shane's go, as review-api deploys do

  Rule: EM10. raea news, the list (11-12)
    Background:
      Given the list is D1's mail_list (0044), one row per address: pending, subscribed or unsubscribed, the exact
        words the person agreed to, where (site, account, or Resend's own page) and their country, and when they asked,
        confirmed and unsubscribed. That row is the consent record
      And Resend holds the same list as contacts on the "raea news" topic (opt-out by default, public, so Resend's
        unsubscribe page shows it) and segment, written through by src/mail/list.ts with a full-access key
        (RESEND_CONTACTS_KEY; the sending key can't touch contacts). Broadcasts go to that segment and topic
    Scenario: EM10a. Nobody is on it without asking
      Then the box by the sign-up is never ticked for anyone, and raea-api refuses a sign-up without consent: true
        (400 consent_required)
      And the words by the box are list.ts CONSENT, kept with each row: "Send me raea news: new releases and what's
        new, about once a month. I can unsubscribe at any time."
      And the GitHub-noreply address of an account is never put on it (it reads no mail)
    Scenario: EM10b. The site's sign-up confirms by mail (double opt-in)
      When someone enters an address and ticks the box in raea.io's footer (news-signup.tsx)
      Then raea-api answers the same whatever the address, keeps a pending row, and mails a confirmation (at most 3 a
        day to one address); the address goes on the list, and to Resend, only when that mail's button is pressed
        (GET /mail/confirm shows the button, POST confirms, so a mail scanner opening the link does nothing)
      And a pending row is deleted after 7 days by the cron
    Scenario: EM10c. The account's switches
      Then the Overview's Email card (email-prefs.tsx) shows raea news (off until turned on, with CONSENT's words),
        invites, review requests and activity (on, each can be turned off), and sign-in and security (always on,
        locked), saved as each changes through GET/PUT /v1/mail/prefs
      And a signed-in person turning news on needs no confirming mail: the address is the account's own
      And an address raea can't mail (GitHub's private one, or one that bounced) says so and locks the switches
    Scenario: EM10d. Out at any time, from anywhere
      Then turning it off in the account, Resend's unsubscribe link in a broadcast ({{{RESEND_UNSUBSCRIBE_URL}}}) and
        Resend's dashboard all end in the same row: the account writes through to Resend, and Resend's
        contact.updated and contact.topics.updated webhooks write back to D1 (never back to Resend again)
      And an address that unsubscribed stays unsubscribed until that person turns it on again themselves
      And if Resend is down when a change is made, D1 keeps it and the hourly cron writes it through (synced_at)
    Scenario: EM10e. Tests
      Then test/mail.sh covers double opt-in, Resend's unsubscribe writing back, the account's prefs and an outage
        caught up by the cron; test/mail-web.sh covers the footer's sign-up, signing in with an emailed code and the
        Email card in Chrome at desktop and phone widths
