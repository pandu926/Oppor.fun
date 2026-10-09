#!/usr/bin/env python3
"""Generate the checked-in API contract; --check verifies it and the routed path inventory."""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
def ref(name): return {"$ref": f"#/components/schemas/{name}"}
def obj(properties, required=()): return {"type": "object", "properties": properties, "required": list(required), "additionalProperties": False}
def nullable(schema): return {"anyOf": [schema, {"type": "null"}]}
TEXT = {"type": "string"}
UUID = {"type": "string", "format": "uuid"}
ADDRESS = {"type": "string", "pattern": "^0x[0-9a-fA-F]{40}$"}
HASH = {"type": "string", "pattern": "^0x[0-9a-fA-F]{64}$"}
AMOUNT = {"type": "string", "pattern": "^(0|[1-9][0-9]{0,77})$", "description": "Canonical decimal uint256 base units; runtime also enforces the uint256 maximum."}
VERSION = {"type": "integer", "format": "int64", "minimum": 0}
TIME = {"type": "string", "format": "date-time", "description": "RFC3339 timestamp; campaign scheduling inputs require whole-second precision."}
URI = {"type": "string", "format": "uri", "maxLength": 2048}
schemas = {
    "Error": obj({"error": obj({"code": TEXT, "message": TEXT, "request_id": UUID}, ["code", "message", "request_id"])}, ["error"]),
    "VersionInput": obj({"expected_version": VERSION}, ["expected_version"]),
    "Task": obj({"task_type": {"type": "string", "enum": ["X_REPOST", "X_LIKE", "X_COMMENT", "X_TAG", "DISCORD_JOIN", "CUSTOM"]}, "target_url": URI, "instructions": {"type": "string", "maxLength": 2000, "default": ""}, "required": {"type": "boolean", "default": True}}, ["task_type", "target_url"]),
    "Reward": obj({"asset_kind": {"type": "string", "enum": ["ERC20", "ERC721", "ERC1155"]}, "token_address": ADDRESS, "amount_base_units": AMOUNT, "token_id": {**AMOUNT, "default": "0"}, "nft_inventory": {"type": "array", "items": AMOUNT, "maxItems": 100, "uniqueItems": True, "default": []}}, ["asset_kind", "token_address", "amount_base_units"]),
    "Distribution": obj({"mode": {"type": "string", "enum": ["ALL_ELIGIBLE", "RAFFLE"]}, "winner_count": {"type": "integer", "minimum": 0, "maximum": 100000, "default": 0}, "capacity": {"type": "integer", "minimum": 0, "maximum": 100000, "default": 0}, "registration_limit": {"type": "integer", "minimum": 1, "maximum": 100000, "default": 100000}, "allocation_policy": {"type": "string", "enum": ["EQUAL_POOL", "FIXED_REWARD"], "default": "EQUAL_POOL"}, "reward_per_recipient": nullable(AMOUNT)}, ["mode"]),
    "CampaignInput": obj({"title": {"type": "string", "minLength": 3, "maxLength": 100}, "description": {"type": "string", "maxLength": 5000, "default": ""}, "chain_id": AMOUNT, "reward": ref("Reward"), "distribution": ref("Distribution"), "start_at": TIME, "cutoff_at": TIME, "review_deadline": TIME, "claim_deadline": TIME, "refund_recipient": nullable(ADDRESS), "tasks": {"type": "array", "items": ref("Task"), "minItems": 1, "maxItems": 20}}, ["title", "chain_id", "reward", "distribution", "start_at", "cutoff_at", "review_deadline", "claim_deadline", "tasks"]),
    "UpdateInput": obj({"expected_version": VERSION, "campaign": ref("CampaignInput")}, ["expected_version", "campaign"]),
    "TaskEdit": obj({"expected_version": VERSION, "task": ref("Task")}, ["expected_version", "task"]),
    "ChallengeInput": obj({"wallet": ADDRESS, "chain_id": AMOUNT}, ["wallet", "chain_id"]),
    "VerifyInput": obj({"message": {"type": "string", "maxLength": 4096}, "signature": {"type": "string", "maxLength": 8194, "pattern": "^0x[0-9a-fA-F]+$"}}, ["message", "signature"]),
    "RegisterInput": obj({"x_username": nullable({"type": "string", "maxLength": 64}), "discord_username": nullable({"type": "string", "maxLength": 100})}),
    "EvidenceInput": obj({"expected_version": VERSION, "text": nullable({"type": "string", "maxLength": 2000}), "url": nullable(URI), "upload_id": nullable(UUID)}, ["expected_version"]),
    "ReviewInput": obj({"expected_version": VERSION, "decision": {"type": "string", "enum": ["ELIGIBLE", "DISQUALIFIED"]}, "reason": {"type": "string", "minLength": 1, "maxLength": 2000}}, ["expected_version", "decision", "reason"]),
    "ClaimInput": obj({"claim_index": {"type": "integer", "minimum": 0, "maximum": 99999}}, ["claim_index"]),
    "TrackInput": obj({"tx_hash": HASH, "kind": {"type": "string", "enum": ["CREATE", "FUND", "ACTIVATE", "FINALIZE", "CLAIM", "CANCEL", "SWEEP"]}}, ["tx_hash", "kind"]),
    "UploadInput": obj({"campaign_id": UUID, "content_type": {"type": "string", "enum": ["image/png", "image/jpeg", "image/webp"]}, "size_bytes": {"type": "integer", "minimum": 1, "maximum": 5242880}}, ["campaign_id", "content_type", "size_bytes"]),
    "PreparedTransaction": obj({"chain_id": AMOUNT, "to": ADDRESS, "data": {"type": "string", "pattern": "^0x[0-9a-fA-F]*$"}, "value": AMOUNT, "expected_sender": ADDRESS, "intent": TEXT, "estimated_gas": nullable(AMOUNT), "expires_at": TIME, "config_hash": HASH, "distribution_root": HASH, "manifest_hash": HASH}, ["chain_id", "to", "data", "value", "expected_sender", "intent", "expires_at"]),
    "JsonResult": {"type": "object", "description": "Endpoint response documented in API.md; amounts and IDs follow the shared scalar conventions."},
}
def arr(schema): return {"type": "array", "items": schema}
def result(properties, optional=()): return obj(properties, [key for key in properties if key not in optional])
COUNT = {"type": "integer", "minimum": 0}
schemas["PreparedTransaction"]["properties"]["simulation"] = TEXT
schemas.update({
    "AdminProfile": result({"user_id":UUID,"wallet":ADDRESS,"role":{"const":"ADMIN"},"reauthenticate_at":TIME,"permissions":arr(TEXT)}),
    "ModerationInput": obj({"expected_version":VERSION,"hidden":{"type":"boolean"},"reason":{"type":"string","minLength":1,"maxLength":2000}},["expected_version","hidden","reason"]),
    "SuspensionInput": obj({"expected_version":VERSION,"suspended":{"type":"boolean"},"reason":{"type":"string","minLength":1,"maxLength":2000}},["expected_version","suspended","reason"]),
    "RevokeInput": obj({"expected_version":VERSION,"reason":{"type":"string","minLength":1,"maxLength":2000}},["expected_version","reason"]),
    "ModerationResult": result({"campaign_id":UUID,"hidden":{"type":"boolean"},"version":VERSION}),
    "SuspensionResult": result({"user_id":UUID,"suspended":{"type":"boolean"},"version":VERSION,"revoked_sessions":COUNT}),
    "RevokeResult": result({"user_id":UUID,"version":VERSION,"revoked_sessions":COUNT}),
    "AdminUser": result({"id":UUID,"wallet":ADDRESS,"created_at":TIME,"suspended":{"type":"boolean"},"version":VERSION,"suspension_reason":nullable(TEXT),"suspension_updated_at":nullable(TIME)}),
    "ModerationState": result({"hidden":{"type":"boolean"},"version":VERSION,"reason":nullable(TEXT),"updated_at":nullable(TIME)}),
    "AdminCampaign": result({"campaign":ref("Campaign"),"moderation":ref("ModerationState")}),
    "AdminAudit": result({"id":UUID,"actor_id":nullable(UUID),"campaign_id":nullable(UUID),"action":TEXT,"metadata":{"type":"object","additionalProperties":True},"created_at":TIME}),
    "AdminJob": result({"id":UUID,"campaign_id":UUID,"kind":TEXT,"status":{"type":"string","enum":["PENDING","RUNNING","FAILED","SUCCEEDED"]},"attempts":COUNT,"available_at":TIME,"locked_until":nullable(TIME),"last_error":nullable(TEXT),"created_at":TIME}),
    "AdminUserPage": result({"items":arr(ref("AdminUser")),"next_cursor":nullable(TEXT)}),
    "AdminCampaignPage": result({"items":arr(ref("AdminCampaign")),"next_cursor":nullable(TEXT)}),
    "AdminAuditPage": result({"items":arr(ref("AdminAudit")),"next_cursor":nullable(TEXT)}),
    "AdminJobPage": result({"items":arr(ref("AdminJob")),"next_cursor":nullable(TEXT)}),
    "AdminCounts": result({"users":COUNT,"suspended_users":COUNT,"campaigns":COUNT,"hidden_campaigns":COUNT,"public_campaigns":COUNT,"entries":COUNT,"claimed_allocations":COUNT,"jobs":obj({status:COUNT for status in ["PENDING","RUNNING","FAILED","SUCCEEDED"]})}),
    "AdminIndexer": result({"next_block":AMOUNT,"observed_safe_head":nullable(AMOUNT),"updated_at":TIME,"halted_reason":nullable(TEXT),"healthy":{"type":"boolean"}}),
    "AdminStats": result({"counts":ref("AdminCounts"),"indexer":nullable(ref("AdminIndexer")),"generated_at":TIME}),
    "HealthResult": result({"status": TEXT}),
    "ChallengeResult": result({"message": TEXT, "expires_at": TIME}),
    "Profile": result({"user_id": UUID, "wallet": ADDRESS}),
    "SessionResult": result({"user_id": UUID, "wallet": ADDRESS, "csrf_token": TEXT, "expires_in": COUNT}),
    "LogoutResult": result({"logged_out": {"type": "boolean"}}),
    "TaskView": result({**schemas["Task"]["properties"], "id": UUID, "position": COUNT}),
    "LockedRules": result({"schema_version": COUNT, "campaign_key": HASH, "chain_id": AMOUNT, "creator": ADDRESS, "refund_recipient": ADDRESS, "reward": ref("Reward"), "distribution": ref("Distribution"), "task_ids": arr(UUID), "tasks": arr(ref("Task")), "starts_at": AMOUNT, "cutoff_at": AMOUNT, "review_deadline": AMOUNT, "claim_deadline": AMOUNT, "verification_mode": TEXT, "raffle_algorithm": TEXT, "raffle_seed_commitment": nullable(HASH), "claim_policy": TEXT, "refund_policy": TEXT}),
    "Campaign": result({"id": UUID, "campaign_key": HASH, "creator_wallet": ADDRESS, "chain_id": AMOUNT, "escrow_address": nullable(ADDRESS), "title": TEXT, "description": TEXT, "status": TEXT, "reward": ref("Reward"), "distribution": ref("Distribution"), "start_at": TIME, "cutoff_at": TIME, "review_deadline": TIME, "claim_deadline": TIME, "registered_count": COUNT, "registration_limit": COUNT, "config_hash": nullable(HASH), "rules_hash": nullable(HASH), "rules": nullable(ref("LockedRules")), "version": VERSION, "verification_mode": TEXT, "created_at": TIME, "tasks": arr(ref("TaskView"))}, ["tasks"]),
    "CampaignPage": result({"items": arr(ref("Campaign")), "next_cursor": nullable(TEXT)}),
    "TaskMutation": result({"task_id": UUID, "version": VERSION}),
    "DeleteTaskResult": result({"deleted": {"type": "boolean"}, "version": VERSION}),
    "ConfigResult": result({"campaign_id": UUID, "config_hash": HASH, "rules_hash": HASH, "rules": ref("LockedRules"), "version": VERSION}),
    "Entry": result({"id": UUID, "campaign_id": UUID, "payout_wallet": ADDRESS, "slot_number": COUNT, "status": TEXT, "version": VERSION, "submitted_at": nullable(TIME), "review": result({"decision": TEXT, "reason": TEXT})}, ["review"]),
    "EntryPage": result({"items": arr(ref("Entry")), "next_cursor": nullable(UUID)}),
    "EntryMutation": result({"entry_id": UUID, "status": TEXT, "version": VERSION}),
    "EvidenceMutation": result({"entry_id": UUID, "task_id": UUID, "revision": COUNT, "version": VERSION, "status": TEXT}),
    "Evidence": result({"text": nullable(TEXT), "url": nullable(URI), "upload_id": nullable(UUID)}),
    "EvidenceItem": result({"task_id": UUID, "revision": COUNT, "evidence": ref("Evidence"), "image_url": nullable(URI)}),
    "EvidenceResult": result({"entry": ref("Entry"), "x_username_declared": nullable(TEXT), "discord_username_declared": nullable(TEXT), "evidence": arr(ref("EvidenceItem"))}),
    "EligibilityResult": result({"campaign_id": UUID, "status": TEXT, "snapshot_hash": HASH, "eligible_count": COUNT, "version": VERSION}),
    "AllocationPreview": result({"root": HASH, "manifest_hash": HASH, "leaf_count": COUNT, "allocated_quantity": AMOUNT, "manifest_url": URI, "trust_model": TEXT}),
    "Results": result({"eligibility_hash": HASH, "eligible_snapshot_url": URI, "raffle_transcript_url": nullable(URI), "root": HASH, "manifest_hash": HASH, "leaf_count": COUNT, "manifest_url": URI}),
    "ManifestResult": result({"manifest_hash": HASH, "url": URI, "expires_in": COUNT}),
    "Allocation": result({"index": AMOUNT, "recipient": ADDRESS, "token_id": AMOUNT, "quantity": AMOUNT, "proof": arr(HASH), "claim_tx_hash": nullable(HASH), "claimed_at": nullable(TIME)}),
    "AllocationResult": result({"campaign_id": UUID, "escrow": ADDRESS, "claim_deadline": TIME, "allocations": arr(ref("Allocation"))}),
    "TrackResult": result({"tx_hash": HASH, "status": TEXT, "note": TEXT}),
    "UploadPolicy": result({"url": URI, "method": {"const": "POST"}, "fields": {"type": "object", "additionalProperties": TEXT}, "expires_in": COUNT, "instructions": TEXT}),
    "UploadResult": result({"upload_id": UUID, "upload": ref("UploadPolicy")}),
    "UploadComplete": result({"upload_id": UUID, "status": {"const": "COMPLETE"}}),
    "FundingResult": result({"approvals": arr(ref("PreparedTransaction")), "fund": ref("PreparedTransaction"), "remaining_quantity": AMOUNT, "instructions": TEXT}),
})
# path, method, summary, session authentication, request schema, idempotency, response schema
catalog = [
    ("/admin/me","get","Read admin permissions; allowlisted wallet and fresh session required",True,None,False,"AdminProfile"),
    ("/admin/stats","get","Read platform counters and indexer status; administrator only",True,None,False,"AdminStats"),
    ("/admin/campaigns","get","Browse all campaigns and moderation state; administrator only",True,None,False,"AdminCampaignPage"),
    ("/admin/campaigns/{id}/moderation","post","Hide/unhide discovery and new registration without changing claim rights",True,"ModerationInput",True,"ModerationResult"),
    ("/admin/users","get","Browse users without exposing session credentials; administrator only",True,None,False,"AdminUserPage"),
    ("/admin/users/{id}/suspension","post","Suspend/restore a user and revoke sessions on suspension",True,"SuspensionInput",True,"SuspensionResult"),
    ("/admin/users/{id}/revoke-sessions","post","Revoke a user's current sessions; administrator only",True,"RevokeInput",True,"RevokeResult"),
    ("/admin/audit-logs","get","Read append-only audit history; administrator only",True,None,False,"AdminAuditPage"),
    ("/admin/jobs","get","Monitor durable jobs; administrator only",True,None,False,"AdminJobPage"),
    ("/health/live", "get", "Process liveness", False, None, False, "JsonResult"),
    ("/health/ready", "get", "Database, Redis, and indexer readiness", False, None, False, "JsonResult"),
    ("/metrics", "get", "Prometheus metrics; restrict at the reverse proxy", False, None, False, None),
    ("/auth/challenge", "post", "Issue a wallet sign-in challenge", False, "ChallengeInput", False, "JsonResult"),
    ("/auth/verify", "post", "Verify a signature and issue a session cookie", False, "VerifyInput", False, "JsonResult"),
    ("/auth/logout", "post", "Revoke the current session", True, None, False, "JsonResult"),
    ("/me", "get", "Read the authenticated wallet profile", True, None, False, "JsonResult"),
    ("/campaigns", "get", "Browse public campaigns using cursor pagination", False, None, False, "JsonResult"),
    ("/campaigns", "post", "Create a campaign draft", True, "CampaignInput", True, "JsonResult"),
    ("/campaigns/mine", "get", "Browse creator-owned campaigns", True, None, False, "JsonResult"),
    ("/campaigns/{id}", "get", "Read a campaign and ordered tasks; drafts require creator session", False, None, False, "JsonResult"),
    ("/campaigns/{id}", "patch", "Replace a complete draft with optimistic concurrency", True, "UpdateInput", True, "JsonResult"),
    ("/campaigns/{id}/tasks", "post", "Append a task to a draft", True, "TaskEdit", True, "JsonResult"),
    ("/campaigns/{id}/tasks/{task_id}", "patch", "Update a draft task", True, "TaskEdit", True, "JsonResult"),
    ("/campaigns/{id}/tasks/{task_id}", "delete", "Delete a draft task", True, "VersionInput", True, "JsonResult"),
    ("/campaigns/{id}/lock-config", "post", "Lock rules, deadlines, and raffle commitment", True, "VersionInput", True, "JsonResult"),
    ("/campaigns/{id}/entries", "get", "List entries; creator only", True, None, False, "JsonResult"),
    ("/campaigns/{id}/entries", "post", "Register a participant using an atomic capacity slot", True, "RegisterInput", True, "JsonResult"),
    ("/campaigns/{id}/my-entry", "get", "Read the current participant's entry and latest review", True, None, False, "JsonResult"),
    ("/campaigns/{id}/my-entry/submissions/{task_id}", "put", "Save evidence and require explicit resubmission", True, "EvidenceInput", True, "JsonResult"),
    ("/campaigns/{id}/my-entry/submit", "post", "Submit latest required evidence before cutoff", True, "VersionInput", True, "JsonResult"),
    ("/campaigns/{id}/entries/{entry_id}/evidence", "get", "Read private evidence; owner or campaign creator only", True, None, False, "JsonResult"),
    ("/campaigns/{id}/entries/{entry_id}/review", "post", "Append a creator review after cutoff", True, "ReviewInput", True, "JsonResult"),
    ("/campaigns/{id}/lock-eligibility", "post", "Freeze reviewed eligibility and enqueue allocations", True, "VersionInput", True, "JsonResult"),
    ("/campaigns/{id}/allocation-preview", "get", "Preview allocation totals and immutable artifact; creator only", True, None, False, "JsonResult"),
    ("/campaigns/{id}/results", "get", "Read final snapshot and raffle artifact URLs", False, None, False, "JsonResult"),
    ("/campaigns/{id}/manifest", "get", "Read final immutable manifest URL and hash", False, None, False, "JsonResult"),
    ("/campaigns/{id}/allocations/{wallet}", "get", "Read a recipient's final allocations and Merkle proofs", False, None, False, "JsonResult"),
    ("/campaigns/{id}/transactions", "post", "Track a user transaction hash; never proof of success", True, "TrackInput", True, "JsonResult"),
    ("/uploads/presign", "post", "Issue an exact-size S3 POST policy for private evidence", True, "UploadInput", True, "JsonResult"),
    ("/uploads/{upload_id}/complete", "post", "Validate and preserve uploaded evidence immutably", True, None, True, "JsonResult"),
]
for intent in ["create", "fund", "activate", "finalize", "claim", "cancel", "sweep"]:
    catalog.append((f"/campaigns/{{id}}/prepare-{intent}", "post", f"Prepare unsigned {intent} transaction; user wallet executes it", True, "ClaimInput" if intent == "claim" else None, False, "JsonResult" if intent == "fund" else "PreparedTransaction"))
response_types = {
    ("/health/live", "get"): "HealthResult", ("/health/ready", "get"): "HealthResult",
    ("/auth/challenge", "post"): "ChallengeResult", ("/auth/verify", "post"): "SessionResult",
    ("/auth/logout", "post"): "LogoutResult", ("/me", "get"): "Profile",
    ("/campaigns", "get"): "CampaignPage", ("/campaigns/mine", "get"): "CampaignPage",
    ("/campaigns", "post"): "Campaign", ("/campaigns/{id}", "get"): "Campaign", ("/campaigns/{id}", "patch"): "Campaign",
    ("/campaigns/{id}/tasks", "post"): "TaskMutation", ("/campaigns/{id}/tasks/{task_id}", "patch"): "TaskMutation",
    ("/campaigns/{id}/tasks/{task_id}", "delete"): "DeleteTaskResult", ("/campaigns/{id}/lock-config", "post"): "ConfigResult",
    ("/campaigns/{id}/entries", "get"): "EntryPage", ("/campaigns/{id}/entries", "post"): "Entry",
    ("/campaigns/{id}/my-entry", "get"): "Entry", ("/campaigns/{id}/my-entry/submissions/{task_id}", "put"): "EvidenceMutation",
    ("/campaigns/{id}/my-entry/submit", "post"): "EntryMutation", ("/campaigns/{id}/entries/{entry_id}/review", "post"): "EntryMutation",
    ("/campaigns/{id}/entries/{entry_id}/evidence", "get"): "EvidenceResult", ("/campaigns/{id}/lock-eligibility", "post"): "EligibilityResult",
    ("/campaigns/{id}/allocation-preview", "get"): "AllocationPreview", ("/campaigns/{id}/results", "get"): "Results",
    ("/campaigns/{id}/manifest", "get"): "ManifestResult", ("/campaigns/{id}/allocations/{wallet}", "get"): "AllocationResult",
    ("/campaigns/{id}/transactions", "post"): "TrackResult", ("/uploads/presign", "post"): "UploadResult",
    ("/uploads/{upload_id}/complete", "post"): "UploadComplete", ("/campaigns/{id}/prepare-fund", "post"): "FundingResult",
}
del schemas["JsonResult"]
paths = {}
for path, method, summary, authenticated, request_schema, idempotent, response_schema in catalog:
    if response_schema == "JsonResult": response_schema = response_types[(path, method)]
    parameters = []
    for name in re.findall(r"\{([^}]+)\}", path):
        parameters.append({"in": "path", "name": name, "required": True, "schema": ADDRESS if name == "wallet" else UUID})
    if method not in ("get", "head"):
        parameters.append({"in": "header", "name": "Origin", "required": True, "schema": URI, "description": "Exact trusted origin configured on the server."})
        if authenticated:
            parameters.append({"in": "header", "name": "X-CSRF-Token", "required": True, "schema": {"type": "string", "minLength": 64, "maxLength": 64}})
    if idempotent:
        parameters.append({"in": "header", "name": "Idempotency-Key", "required": True, "schema": {"type": "string", "pattern": "^[A-Za-z0-9_.-]{8,128}$"}})
    if method == "get" and path in ("/campaigns", "/campaigns/mine"):
        for name, schema in [("limit", {"type": "integer", "minimum": 1, "maximum": 100}), ("cursor", TEXT), ("mode", {"type": "string", "enum": ["ALL_ELIGIBLE", "RAFFLE"]}), ("asset_kind", {"type": "string", "enum": ["ERC20", "ERC721", "ERC1155"]}), ("status", TEXT), ("sort", {"type": "string", "enum": ["newest", "ending"]})]: parameters.append({"in": "query", "name": name, "schema": schema})
    if method == "get" and path == "/campaigns/{id}/entries":
        for name, schema in [("limit", {"type": "integer", "minimum": 1, "maximum": 100}), ("after", UUID), ("status", TEXT)]: parameters.append({"in": "query", "name": name, "schema": schema})
    if method == "get" and path in ("/admin/campaigns","/admin/users","/admin/audit-logs","/admin/jobs"):
        filters={"/admin/campaigns":[("hidden",{"type":"boolean"})],"/admin/users":[("wallet",ADDRESS),("suspended",{"type":"boolean"})],"/admin/audit-logs":[("campaign_id",UUID),("actor_id",UUID)],"/admin/jobs":[("campaign_id",UUID),("status",{"type":"string","enum":["PENDING","RUNNING","FAILED","SUCCEEDED"]})]}
        for name,schema in [("limit",{"type":"integer","minimum":1,"maximum":100}),("cursor",TEXT),*filters[path]]: parameters.append({"in":"query","name":name,"schema":schema})
    success = {"description": "Successful operation", "content": {"application/json": {"schema": ref(response_schema)}}} if response_schema else {"description": "Prometheus metrics", "content": {"text/plain": {"schema": TEXT}}}
    operation = {"operationId": method + "_" + re.sub(r"[^a-zA-Z0-9]+", "_", path).strip("_"), "summary": summary, "tags": [path.split('/')[1]], "security": [{"sessionCookie": []}] if authenticated else [], "parameters": parameters, "responses": {"200": success, "default": {"description": "Error; response includes a request ID", "content": {"application/json": {"schema": ref("Error")}}}}}
    if request_schema:
        operation["requestBody"] = {"required": True, "content": {"application/json": {"schema": ref(request_schema)}}}
    paths.setdefault(path, {})[method] = operation

spec = {"openapi": "3.1.0", "info": {"title": "Oppor API", "version": "0.1.0", "description": "Manual-review campaign promotion and escrow integration. No X APIs or private-key custody."}, "servers": [{"url": "/v1"}], "paths": paths, "components": {"securitySchemes": {"sessionCookie": {"type": "apiKey", "in": "cookie", "name": "__Host-oppor_session", "description": "HttpOnly Secure cookie in production; local development uses oppor_session."}}, "schemas": schemas}}
source = (ROOT / "src/http.rs").read_text()
routed = set(re.findall(r'\.route\(\s*"([^"]+)"', source))
assert routed == set(paths), f"Route inventory mismatch: {routed ^ set(paths)}"
encoded = json.dumps(spec, indent=2) + "\n"
if "--check" in sys.argv:
    assert (ROOT / "openapi.json").read_text() == encoded, "Regenerate openapi.json"
else:
    (ROOT / "openapi.json").write_text(encoded)
print(f"OpenAPI: {len(paths)} paths, {len(catalog)} operations.")
