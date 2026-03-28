# SITEMAP

This document is a planning artifact. It is meant to keep route decisions
small, explicit, and easy to revise before implementation.

## Conventions

| Marker | Meaning |
| --- | --- |
| `Public` | Does not require admin login |
| `Protected` | Requires admin login |
| `Current` | Already exists in the app today |
| `Planned` | Intended route, not implemented yet |

## Current Routes

| Status | Route | Access | Expected User Activity | Notes |
| --- | --- | --- | --- | --- |
| `Current` | `/hello` | `Public` | Confirm the app is up and rendering a localized page | Current prototype / placeholder landing page |
| `Current` | `/preferences/language` | `Public` | Change the active UI language and return to the current page | Form post endpoint, not a standalone page |
| `Current` | `/static/*` | `Public` | Load CSS and other static assets required by the UI | Asset delivery only |

## Planned Routes

These are the smallest useful routes to support an admin-facing application
without over-designing the system up front.

| Status | Route | Access | Expected User Activity | Notes |
| --- | --- | --- | --- | --- |
| `Planned` | `/` | `Public` | Land on the app and get routed to the right starting point | Likely redirect to `/dashboard` when authenticated, otherwise `/login` |
| `Planned` | `/login` | `Public` | Sign in to the admin application | Primary public entry point |
| `Planned` | `/logout` | `Protected` | End the current session safely | Could be POST-only in implementation |
| `Planned` | `/dashboard` | `Protected` | Get an at-a-glance view of work needing attention today | Default authenticated landing page |
| `Planned` | `/clients` | `Protected` | Find, review, and manage people or households served by the organization | List / search page |
| `Planned` | `/clients/:id` | `Protected` | Review one client record and take follow-up actions | Detail page |
| `Planned` | `/services` | `Protected` | Record or review services, visits, or assistance provided | Operational workflow page |
| `Planned` | `/inventory` | `Protected` | Track available goods, supplies, or stock used in assistance workflows | Useful if the organization manages material aid |
| `Planned` | `/reports` | `Protected` | Review summary metrics and export operational data | Reporting surface |
| `Planned` | `/admin/users` | `Protected` | Manage staff/admin accounts and access | Administrative settings |

## Planning Notes

- Public routes should be the exception, not the default.
- Utility endpoints like `/preferences/language` can stay in the sitemap, but
  should be labeled clearly as non-page routes.
- It is reasonable to replace `/hello` with `/` once a real landing flow exists.
- If the scope needs to stay tighter, start with `/login`, `/dashboard`,
  `/clients`, and `/clients/:id`, then add the rest only when a real workflow
  demands them.
