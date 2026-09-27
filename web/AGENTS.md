# Web and hosted-source instructions

Read the root `AGENTS.md`, `docs/PRODUCT-CONTRACT.md`, and
`docs/HOSTED-SOURCE-PLAN-ADDENDUM.md` before protocol, chat, or layout work.

- `parent.mpdviewer.com/index.html` is the owner-supplied provenance snapshot.
  Preserve it byte-for-byte unless a task explicitly authorizes refreshing that
  snapshot with recorded provenance.
- The bundled player wrapper, native compatibility adapter, provenance snapshot,
  and deployed website are distinct. Do not substitute or deploy one for another
  by assumption.
- Keep demo behavior independent of live Twitch media requests.
- Do not add broad message listeners, arbitrary navigation, analytics, or script
  injection into full Twitch pages.
- Website deployment is never implied by a source change or PR.
