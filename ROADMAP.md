# Auth And Access Roadmap

## Summary

This is a living roadmap. The auth foundation below is intentionally specific because it is the first execution target. The milestones after that are deliberately broader sketches that will be refined once implementation reveals the real constraints and tradeoffs.

- Local auth is the initial strategy.
- Admin auth ships first.
- Volunteer access remains on the roadmap but follows the admin foundation.

## Current Auth Foundation

- Use SQLx migrations as the schema source of truth.
- Use Argon2id for admin password hashing.
- Use server-backed sessions with an opaque cookie token and only a token hash stored in PostgreSQL.
- Bootstrap the first admin from environment configuration when no admin users exist.
- Force a password change on the first bootstrap login.
- Support admin-managed password resets by assigning a temporary password and forcing a password change on next login.
- Revoke active sessions when an admin password is reset.
- Protect `/admin` and related routes behind authenticated admin sessions.

## Initial Deliverables

- Login chooser page with Admin and Volunteer cards
- Admin username/password login
- Admin dashboard shell with Registration, Inventory, Volunteer Records, Reporting, and Configuration / Settings entry points
- Password management page for self-service password changes and admin-managed resets
- Audit events for login success, login failure, logout, password changes, and resets

## Follow-On Milestones

These are high-level concepts for now and will be revised as implementation proceeds.

- Volunteer access
  - Last-four phone lookup
  - De-duplication chooser when necessary
  - Activity selection and start-the-clock / enter-hours flow
- Volunteer correction tools
  - Admin audit, edit, and session correction workflows
- Registration
  - Check-in oriented administrative workflow
- Inventory
  - Supply and stock workflows
- Reporting
  - Cross-area summaries and exports
- Configuration / Settings
  - Broader account management
  - Operational defaults
  - Future system configuration
