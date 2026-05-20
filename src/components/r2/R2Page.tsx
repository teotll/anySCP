import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  BarChart3,
  Clock3,
  Cloud,
  Globe2,
  Link,
  Plus,
  RefreshCw,
  Save,
  Shield,
  Trash2,
} from "lucide-react";
import type {
  R2AttachCustomDomainRequest,
  R2Bucket,
  R2CreateBucketRequest,
  R2Json,
  S3Connection,
} from "../../types";
import { CustomSelect } from "../shared/CustomSelect";
import { S3ConnectDialog } from "../s3/S3ConnectDialog";

type R2Tab = "overview" | "cors" | "lifecycle" | "domains" | "metrics";

interface ConfirmAction {
  title: string;
  body: string;
  confirmText: string;
  actionLabel: string;
  onConfirm: () => Promise<void>;
}

interface R2ErrorInfo {
  message: string;
  action?: "edit_connection";
}

const TAB_ITEMS: Array<{ id: R2Tab; label: string; icon: React.ElementType }> = [
  { id: "overview", label: "Overview", icon: Cloud },
  { id: "cors", label: "CORS", icon: Shield },
  { id: "lifecycle", label: "Lifecycle", icon: Clock3 },
  { id: "domains", label: "Domains", icon: Globe2 },
  { id: "metrics", label: "Metrics", icon: BarChart3 },
];

const EMPTY_CORS = `{
  "rules": [
    {
      "allowed": {
        "methods": ["GET"],
        "origins": ["https://example.com"],
        "headers": ["*"]
      },
      "exposeHeaders": [],
      "maxAgeSeconds": 3600
    }
  ]
}`;

const EMPTY_LIFECYCLE = `{
  "rules": [
    {
      "id": "expire-temp",
      "enabled": true,
      "conditions": { "prefix": "tmp/" },
      "deleteObjectsTransition": {
        "condition": {
          "type": "Age",
          "maxAge": 2592000
        }
      }
    }
  ]
}`;

function errorInfo(err: unknown, fallback: string): R2ErrorInfo {
  if (!err || typeof err !== "object") return { message: fallback };
  const maybeError = err as { kind?: string; message?: string; code?: number | null };
  const message = maybeError.message ? String(maybeError.message) : "";
  const codeSuffix =
    typeof maybeError.code === "number" ? ` (Cloudflare code ${maybeError.code})` : "";

  if (maybeError.kind === "missing_api_token") {
    return {
      message:
        "This R2 connection is missing a Cloudflare API token. Edit the connection and add an R2 admin token.",
      action: "edit_connection",
    };
  }
  if (maybeError.kind === "missing_account_id") {
    return {
      message:
        "This R2 connection is missing its Cloudflare Account ID. Edit the connection and add it.",
      action: "edit_connection",
    };
  }
  if (maybeError.kind === "not_r2_connection") {
    return { message: "Select a Cloudflare R2 connection before using the R2 dashboard." };
  }
  if (maybeError.kind === "connection_not_found") {
    return {
      message: "This saved R2 connection no longer exists. Refresh connections and select another one.",
    };
  }
  if (maybeError.kind === "invalid_request") {
    return {
      message: message || "The R2 request is not valid. Check the form values and try again.",
    };
  }
  if (maybeError.kind === "network") {
    return {
      message: message
        ? `Could not reach Cloudflare: ${message}`
        : "Could not reach Cloudflare. Check your network and try again.",
    };
  }
  if (maybeError.kind === "cloudflare_api") {
    if (isCloudflareAuthError(maybeError.code, message)) {
      return {
        message:
          `Cloudflare rejected this token or it lacks the required R2 permissions. ` +
          `Edit the connection and check the token scopes${codeSuffix}.`,
        action: "edit_connection",
      };
    }
    if (isCloudflareRateLimit(maybeError.code, message)) {
      return {
        message: `Cloudflare rate limited this request. Wait a moment and try again${codeSuffix}.`,
      };
    }
    return {
      message:
        message ||
        `Cloudflare rejected the request. Check your token permissions and bucket settings${codeSuffix}.`,
    };
  }
  if (maybeError.kind === "decode") {
    return {
      message: message
        ? `Cloudflare returned an unexpected response: ${message}`
        : "Cloudflare returned an unexpected response.",
    };
  }
  if (maybeError.kind === "io") {
    return {
      message: message || "The local credential store or database could not be read.",
    };
  }
  return { message: message || fallback };
}

function isCloudflareAuthError(code: number | null | undefined, message: string) {
  return (
    code === 10000 ||
    code === 10001 ||
    code === 10013 ||
    code === 9103 ||
    code === 9109 ||
    /auth|invalid api token|unauthori[sz]ed|permission|forbidden/i.test(message)
  );
}

function isCloudflareRateLimit(code: number | null | undefined, message: string) {
  return code === 10100 || code === 1015 || /rate.?limit|too many requests|http 429/i.test(message);
}

function prettyJson(value: unknown) {
  return JSON.stringify(value ?? {}, null, 2);
}

function bucketName(bucket: R2Bucket | null) {
  return bucket?.name ?? "";
}

function bucketJurisdiction(bucket: R2Bucket | null) {
  return bucket?.jurisdiction && bucket.jurisdiction !== "default" ? bucket.jurisdiction : null;
}

export function R2Page() {
  const [connections, setConnections] = useState<S3Connection[]>([]);
  const [selectedConnectionId, setSelectedConnectionId] = useState("");
  const [showConnectionDialog, setShowConnectionDialog] = useState(false);
  const [editingConnection, setEditingConnection] = useState<S3Connection | null>(null);
  const [buckets, setBuckets] = useState<R2Bucket[]>([]);
  const [selectedBucketName, setSelectedBucketName] = useState("");
  const [tab, setTab] = useState<R2Tab>("overview");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<R2ErrorInfo | null>(null);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction | null>(null);
  const [confirmationText, setConfirmationText] = useState("");

  const [newBucketName, setNewBucketName] = useState("");
  const [newBucketJurisdiction, setNewBucketJurisdiction] = useState("");
  const [newBucketLocation, setNewBucketLocation] = useState("");
  const [newBucketClass, setNewBucketClass] = useState("Standard");

  const [corsJson, setCorsJson] = useState(EMPTY_CORS);
  const [lifecycleJson, setLifecycleJson] = useState(EMPTY_LIFECYCLE);
  const [managedDomain, setManagedDomain] = useState<R2Json>(null);
  const [customDomains, setCustomDomains] = useState<R2Json>(null);
  const [customDomain, setCustomDomain] = useState("");
  const [customZoneId, setCustomZoneId] = useState("");
  const [customMinTls, setCustomMinTls] = useState("1.2");
  const [metrics, setMetrics] = useState<R2Json>(null);

  const selectedConnection = useMemo(
    () => connections.find((connection) => connection.id === selectedConnectionId) ?? null,
    [connections, selectedConnectionId],
  );
  const selectedBucket = useMemo(
    () => buckets.find((bucket) => bucket.name === selectedBucketName) ?? null,
    [buckets, selectedBucketName],
  );

  const loadConnections = useCallback(async () => {
    const saved = await invoke<S3Connection[]>("s3_list_connections");
    const r2Connections = saved.filter((connection) => connection.provider === "r2");
    setConnections(r2Connections);
    setSelectedConnectionId((current) => (
      current && r2Connections.some((connection) => connection.id === current)
        ? current
        : r2Connections[0]?.id ?? ""
    ));
  }, []);

  const loadBuckets = useCallback(async () => {
    if (!selectedConnectionId) {
      setBuckets([]);
      setSelectedBucketName("");
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const result = await invoke<R2Bucket[]>("r2_list_buckets", {
        connectionId: selectedConnectionId,
      });
      setBuckets(result);
      setSelectedBucketName((current) => (
        current && result.some((bucket) => bucket.name === current)
          ? current
          : result[0]?.name ?? ""
      ));
    } catch (err) {
      setError(errorInfo(err, "Could not load R2 buckets"));
    } finally {
      setLoading(false);
    }
  }, [selectedConnectionId]);

  const loadBucketPolicy = useCallback(async (policy: "cors" | "lifecycle") => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setLoading(true);
    setError(null);
    try {
      const command = policy === "cors" ? "r2_get_cors" : "r2_get_lifecycle";
      const value = await invoke<R2Json>(command, {
        connectionId: selectedConnectionId,
        bucketName: name,
        jurisdiction: bucketJurisdiction(selectedBucket),
      });
      if (policy === "cors") setCorsJson(prettyJson(value));
      else setLifecycleJson(prettyJson(value));
    } catch (err) {
      setError(errorInfo(err, `Could not load ${policy}`));
    } finally {
      setLoading(false);
    }
  }, [selectedBucket, selectedConnectionId]);

  const loadDomains = useCallback(async () => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setLoading(true);
    setError(null);
    try {
      const [managed, custom] = await Promise.all([
        invoke<R2Json>("r2_get_managed_domain", {
          connectionId: selectedConnectionId,
          bucketName: name,
          jurisdiction: bucketJurisdiction(selectedBucket),
        }),
        invoke<R2Json>("r2_list_custom_domains", {
          connectionId: selectedConnectionId,
          bucketName: name,
          jurisdiction: bucketJurisdiction(selectedBucket),
        }),
      ]);
      setManagedDomain(managed);
      setCustomDomains(custom);
    } catch (err) {
      setError(errorInfo(err, "Could not load domains"));
    } finally {
      setLoading(false);
    }
  }, [selectedBucket, selectedConnectionId]);

  const loadMetrics = useCallback(async () => {
    if (!selectedConnectionId) return;
    setLoading(true);
    setError(null);
    try {
      const value = await invoke<R2Json>("r2_get_metrics", {
        connectionId: selectedConnectionId,
      });
      setMetrics(value);
    } catch (err) {
      setError(errorInfo(err, "Could not load R2 metrics"));
    } finally {
      setLoading(false);
    }
  }, [selectedConnectionId]);

  useEffect(() => {
    void loadConnections();
  }, [loadConnections]);

  useEffect(() => {
    void loadBuckets();
  }, [loadBuckets]);

  useEffect(() => {
    if (tab === "cors") void loadBucketPolicy("cors");
    if (tab === "lifecycle") void loadBucketPolicy("lifecycle");
    if (tab === "domains") void loadDomains();
    if (tab === "metrics") void loadMetrics();
  }, [tab, selectedBucketName, loadBucketPolicy, loadDomains, loadMetrics]);

  const createBucket = async () => {
    if (!selectedConnectionId || !newBucketName.trim()) return;
    setSaving(true);
    setError(null);
    try {
      const request: R2CreateBucketRequest = {
        name: newBucketName.trim(),
        jurisdiction: newBucketJurisdiction || null,
        locationHint: newBucketLocation || null,
        storageClass: newBucketClass || null,
      };
      await invoke("r2_create_bucket", {
        connectionId: selectedConnectionId,
        request,
      });
      setNewBucketName("");
      await loadBuckets();
    } catch (err) {
      setError(errorInfo(err, "Could not create bucket"));
    } finally {
      setSaving(false);
    }
  };

  const deleteBucket = async () => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setConfirmAction({
      title: "Delete Bucket",
      body: `This permanently deletes the R2 bucket "${name}". Cloudflare only allows deleting empty buckets; if the bucket still contains objects, the request will fail.`,
      confirmText: name,
      actionLabel: "Delete Bucket",
      onConfirm: async () => {
        setSaving(true);
        setError(null);
        try {
          await invoke("r2_delete_bucket", {
            connectionId: selectedConnectionId,
            bucketName: name,
            confirmName: name,
            jurisdiction: bucketJurisdiction(selectedBucket),
          });
          await loadBuckets();
        } catch (err) {
          setError(errorInfo(err, "Could not delete bucket"));
        } finally {
          setSaving(false);
        }
      },
    });
  };

  const savePolicy = async (policy: "cors" | "lifecycle") => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setSaving(true);
    setError(null);
    try {
      const parsed = JSON.parse(policy === "cors" ? corsJson : lifecycleJson);
      await invoke(policy === "cors" ? "r2_put_cors" : "r2_put_lifecycle", {
        connectionId: selectedConnectionId,
        bucketName: name,
        jurisdiction: bucketJurisdiction(selectedBucket),
        policy: parsed,
      });
      await loadBucketPolicy(policy);
    } catch (err) {
      setError(err instanceof SyntaxError ? { message: err.message } : errorInfo(err, `Could not save ${policy}`));
    } finally {
      setSaving(false);
    }
  };

  const deleteCors = async () => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setConfirmAction({
      title: "Delete CORS Policy",
      body: `This removes the CORS policy from "${name}". Browser clients that depend on this policy may stop working immediately.`,
      confirmText: name,
      actionLabel: "Delete CORS",
      onConfirm: async () => {
        setSaving(true);
        setError(null);
        try {
          await invoke("r2_delete_cors", {
            connectionId: selectedConnectionId,
            bucketName: name,
            jurisdiction: bucketJurisdiction(selectedBucket),
          });
          setCorsJson(EMPTY_CORS);
        } catch (err) {
          setError(errorInfo(err, "Could not delete CORS policy"));
        } finally {
          setSaving(false);
        }
      },
    });
  };

  const deleteLifecycle = async () => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setConfirmAction({
      title: "Delete Lifecycle Rules",
      body: `This removes all lifecycle rules from "${name}". Existing automatic expiration and transition behavior will stop.`,
      confirmText: name,
      actionLabel: "Delete Lifecycle",
      onConfirm: async () => {
        setSaving(true);
        setError(null);
        try {
          await invoke("r2_delete_lifecycle", {
            connectionId: selectedConnectionId,
            bucketName: name,
            jurisdiction: bucketJurisdiction(selectedBucket),
          });
          setLifecycleJson(EMPTY_LIFECYCLE);
        } catch (err) {
          setError(errorInfo(err, "Could not delete lifecycle rules"));
        } finally {
          setSaving(false);
        }
      },
    });
  };

  const updateManagedDomain = async (enabled: boolean) => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name) return;
    setSaving(true);
    setError(null);
    try {
      const value = await invoke<R2Json>("r2_update_managed_domain", {
        connectionId: selectedConnectionId,
        bucketName: name,
        jurisdiction: bucketJurisdiction(selectedBucket),
        enabled,
      });
      setManagedDomain(value);
    } catch (err) {
      setError(errorInfo(err, "Could not update r2.dev domain"));
    } finally {
      setSaving(false);
    }
  };

  const attachCustomDomain = async () => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name || !customDomain.trim() || !customZoneId.trim()) return;
    setSaving(true);
    setError(null);
    try {
      const request: R2AttachCustomDomainRequest = {
        domain: customDomain.trim(),
        zoneId: customZoneId.trim(),
        enabled: true,
        minTls: customMinTls || null,
      };
      await invoke("r2_attach_custom_domain", {
        connectionId: selectedConnectionId,
        bucketName: name,
        jurisdiction: bucketJurisdiction(selectedBucket),
        request,
      });
      setCustomDomain("");
      await loadDomains();
    } catch (err) {
      setError(errorInfo(err, "Could not attach custom domain"));
    } finally {
      setSaving(false);
    }
  };

  const deleteCustomDomain = async (domain: string) => {
    const name = bucketName(selectedBucket);
    if (!selectedConnectionId || !name || !domain) return;
    setConfirmAction({
      title: "Delete Custom Domain",
      body: `This detaches "${domain}" from "${name}". Traffic to this domain may stop routing to the bucket.`,
      confirmText: domain,
      actionLabel: "Delete Domain",
      onConfirm: async () => {
        setSaving(true);
        setError(null);
        try {
          await invoke("r2_delete_custom_domain", {
            connectionId: selectedConnectionId,
            bucketName: name,
            domain,
            jurisdiction: bucketJurisdiction(selectedBucket),
          });
          await loadDomains();
        } catch (err) {
          setError(errorInfo(err, "Could not delete custom domain"));
        } finally {
          setSaving(false);
        }
      },
    });
  };

  const customDomainRows = Array.isArray((customDomains as { domains?: unknown[] } | null)?.domains)
    ? (customDomains as { domains: Array<Record<string, unknown>> }).domains
    : [];
  const managedEnabled = Boolean((managedDomain as { enabled?: unknown } | null)?.enabled);
  const confirmationMatches = Boolean(confirmAction && confirmationText === confirmAction.confirmText);

  const closeConfirm = () => {
    setConfirmAction(null);
    setConfirmationText("");
  };

  const confirmDestructiveAction = async () => {
    if (!confirmAction || !confirmationMatches) return;
    const action = confirmAction;
    closeConfirm();
    await action.onConfirm();
  };

  return (
    <>
      <div className="flex h-full min-h-0 bg-bg-base">
        <aside className="w-[280px] shrink-0 border-r border-border bg-bg-surface/60 flex flex-col min-h-0">
          <div className="p-4 border-b border-border flex items-center justify-between gap-2">
            <div className="min-w-0">
              <h1 className="text-[length:var(--text-lg)] font-semibold text-text-primary">R2</h1>
              <p className="text-[length:var(--text-xs)] text-text-muted truncate">
                {selectedConnection?.label ?? "No connection"}
              </p>
            </div>
            <button
              type="button"
              onClick={() => setShowConnectionDialog(true)}
              className="h-8 w-8 rounded-lg border border-border bg-bg-overlay text-text-secondary hover:text-text-primary hover:border-border-focus flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              aria-label="New R2 connection"
              title="New R2 connection"
            >
              <Plus size={15} />
            </button>
          </div>

          <div className="p-3 border-b border-border">
            <CustomSelect
              value={selectedConnectionId}
              onChange={setSelectedConnectionId}
              placeholder="Select R2 connection"
              options={connections.map((connection) => ({ value: connection.id, label: connection.label }))}
            />
          </div>

          <div className="p-3 border-b border-border">
            <div className="flex gap-2">
              <input
                value={newBucketName}
                onChange={(e) => setNewBucketName(e.target.value)}
                placeholder="new-bucket"
                className="min-w-0 flex-1 rounded-lg bg-bg-base border border-border px-3 py-2 text-[length:var(--text-xs)] text-text-primary placeholder:text-text-muted outline-none focus:border-border-focus focus:ring-2 focus:ring-ring"
              />
              <button
                type="button"
                onClick={() => void createBucket()}
                disabled={!selectedConnectionId || !newBucketName.trim() || saving}
                className="h-8 w-8 rounded-lg bg-accent text-text-inverse disabled:opacity-50 flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                aria-label="Create bucket"
                title="Create bucket"
              >
                <Plus size={14} />
              </button>
            </div>
            <div className="grid grid-cols-2 gap-2 mt-2">
              <CustomSelect
                value={newBucketJurisdiction}
                onChange={setNewBucketJurisdiction}
                placeholder="Jurisdiction"
                options={[
                  { value: "", label: "Default" },
                  { value: "eu", label: "EU" },
                  { value: "fedramp", label: "FedRAMP" },
                ]}
              />
              <CustomSelect
                value={newBucketLocation}
                onChange={setNewBucketLocation}
                placeholder="Location"
                options={[
                  { value: "", label: "Auto" },
                  { value: "enam", label: "ENAM" },
                  { value: "wnam", label: "WNAM" },
                  { value: "weur", label: "WEUR" },
                  { value: "eeur", label: "EEUR" },
                  { value: "apac", label: "APAC" },
                  { value: "oc", label: "OC" },
                ]}
              />
            </div>
            <div className="mt-2">
              <CustomSelect
                value={newBucketClass}
                onChange={setNewBucketClass}
                options={[
                  { value: "Standard", label: "Standard" },
                  { value: "InfrequentAccess", label: "Infrequent Access" },
                ]}
              />
            </div>
          </div>

          <div className="flex items-center justify-between px-3 py-2 border-b border-border">
            <span className="text-[length:var(--text-xs)] font-semibold uppercase tracking-widest text-text-muted">
              Buckets
            </span>
            <button
              type="button"
              onClick={() => void loadBuckets()}
              disabled={!selectedConnectionId || loading}
              className="h-7 w-7 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-overlay flex items-center justify-center disabled:opacity-50"
              aria-label="Refresh buckets"
              title="Refresh buckets"
            >
              <RefreshCw size={13} className={loading ? "animate-spin" : ""} />
            </button>
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto p-2">
            {buckets.map((bucket) => (
              <button
                key={`${bucket.jurisdiction ?? "default"}:${bucket.name}`}
                type="button"
                onClick={() => setSelectedBucketName(bucket.name ?? "")}
                className={[
                  "w-full text-left px-3 py-2 rounded-lg border transition-colors duration-[var(--duration-fast)]",
                  bucket.name === selectedBucketName
                    ? "bg-accent/15 border-accent/40 text-accent"
                    : "border-transparent text-text-secondary hover:bg-bg-overlay hover:text-text-primary",
                ].join(" ")}
              >
                <div className="text-[length:var(--text-sm)] font-medium truncate">{bucket.name}</div>
                <div className="text-[length:var(--text-2xs)] text-text-muted truncate">
                  {bucket.storage_class ?? "Standard"} · {bucket.jurisdiction ?? "default"}
                </div>
              </button>
            ))}
            {!loading && buckets.length === 0 && (
              <div className="px-3 py-8 text-center text-[length:var(--text-sm)] text-text-muted">
                No buckets
              </div>
            )}
          </div>
        </aside>

        <main className="flex-1 min-w-0 min-h-0 overflow-y-auto">
          <div className="max-w-6xl mx-auto px-6 py-5 flex flex-col gap-4">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <h2 className="text-[length:var(--text-lg)] font-semibold text-text-primary truncate">
                  {selectedBucketName || "Account"}
                </h2>
                <p className="text-[length:var(--text-xs)] text-text-muted truncate">
                  {selectedConnection?.r2_account_id ? `Account: ${selectedConnection.r2_account_id}` : "Cloudflare R2"}
                </p>
              </div>
              {selectedBucket && (
                <button
                  type="button"
                  onClick={() => void deleteBucket()}
                  disabled={saving}
                  className="flex items-center gap-2 h-8 px-3 rounded-lg border border-status-error/40 text-status-error hover:bg-status-error/10 disabled:opacity-50 text-[length:var(--text-xs)] font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <Trash2 size={13} />
                  Delete
                </button>
              )}
            </div>

            {error && (
              <div className="flex items-center justify-between gap-3 rounded-lg border border-status-error/30 bg-status-error/10 px-3 py-2 text-[length:var(--text-sm)] text-status-error">
                <span className="min-w-0 flex-1">{error.message}</span>
                {error.action === "edit_connection" && (
                  <button
                    type="button"
                    onClick={() => {
                      if (selectedConnection) setEditingConnection(selectedConnection);
                      else setShowConnectionDialog(true);
                    }}
                    className="shrink-0 h-7 px-2.5 rounded-md border border-status-error/40 text-status-error hover:bg-status-error/10 text-[length:var(--text-xs)] font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    Edit Connection
                  </button>
                )}
              </div>
            )}

            <div className="flex flex-wrap gap-1 border-b border-border">
              {TAB_ITEMS.map((item) => {
                const Icon = item.icon;
                const active = tab === item.id;
                const disabled = item.id !== "metrics" && !selectedBucket;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setTab(item.id)}
                    disabled={disabled}
                    title={disabled ? "Select a bucket first" : item.label}
                    className={[
                      "flex items-center gap-2 px-3 h-9 text-[length:var(--text-sm)] border-b-2 -mb-px disabled:opacity-40",
                      active
                        ? "border-accent text-accent"
                        : "border-transparent text-text-secondary hover:text-text-primary",
                    ].join(" ")}
                  >
                    <Icon size={14} />
                    {item.label}
                  </button>
                );
              })}
            </div>

            {tab === "overview" && (
              <section className="grid grid-cols-2 gap-3">
                {[
                  ["Name", selectedBucket?.name ?? "No bucket selected"],
                  ["Created", selectedBucket?.creation_date ?? "-"],
                  ["Jurisdiction", selectedBucket?.jurisdiction ?? "default"],
                  ["Location", selectedBucket?.location ?? "-"],
                  ["Storage class", selectedBucket?.storage_class ?? "Standard"],
                  ["Connection", selectedConnection?.label ?? "-"],
                ].map(([label, value]) => (
                  <div key={label} className="rounded-lg border border-border bg-bg-surface px-4 py-3">
                    <div className="text-[length:var(--text-2xs)] uppercase tracking-widest text-text-muted font-semibold">{label}</div>
                    <div className="mt-1 text-[length:var(--text-sm)] text-text-primary break-words">{value}</div>
                  </div>
                ))}
              </section>
            )}

            {tab === "cors" && (
              <JsonEditor
                title="CORS Policy"
                value={corsJson}
                onChange={setCorsJson}
                onSave={() => void savePolicy("cors")}
                onDelete={() => void deleteCors()}
                saving={saving}
              />
            )}

            {tab === "lifecycle" && (
              <JsonEditor
                title="Lifecycle Rules"
                value={lifecycleJson}
                onChange={setLifecycleJson}
                onSave={() => void savePolicy("lifecycle")}
                onDelete={() => void deleteLifecycle()}
                saving={saving}
              />
            )}

            {tab === "domains" && (
              <section className="flex flex-col gap-4">
                <div className="rounded-lg border border-border bg-bg-surface p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <h3 className="text-[length:var(--text-sm)] font-semibold text-text-primary">r2.dev</h3>
                      <p className="text-[length:var(--text-xs)] text-text-muted">
                        {(managedDomain as { domain?: string } | null)?.domain ?? "Managed domain"}
                      </p>
                    </div>
                    <button
                      type="button"
                      onClick={() => void updateManagedDomain(!managedEnabled)}
                      disabled={saving}
                      className={[
                        "h-8 px-3 rounded-lg text-[length:var(--text-xs)] font-medium border disabled:opacity-50",
                        managedEnabled
                          ? "border-status-connected/40 text-status-connected bg-status-connected/10"
                          : "border-border text-text-secondary hover:text-text-primary hover:bg-bg-overlay",
                      ].join(" ")}
                    >
                      {managedEnabled ? "Enabled" : "Disabled"}
                    </button>
                  </div>
                  <pre className="mt-3 max-h-52 overflow-auto rounded-lg bg-bg-base border border-border p-3 text-[length:var(--text-xs)] text-text-secondary">
                    {prettyJson(managedDomain)}
                  </pre>
                </div>

                <div className="rounded-lg border border-border bg-bg-surface p-4">
                  <h3 className="text-[length:var(--text-sm)] font-semibold text-text-primary mb-3">Custom Domains</h3>
                  <div className="grid grid-cols-[1fr_1fr_auto_auto] gap-2 mb-3">
                    <input
                      value={customDomain}
                      onChange={(e) => setCustomDomain(e.target.value)}
                      placeholder="files.example.com"
                      className="rounded-lg bg-bg-base border border-border px-3 py-2 text-[length:var(--text-sm)] text-text-primary placeholder:text-text-muted outline-none focus:border-border-focus focus:ring-2 focus:ring-ring"
                    />
                    <input
                      value={customZoneId}
                      onChange={(e) => setCustomZoneId(e.target.value)}
                      placeholder="zone id"
                      className="rounded-lg bg-bg-base border border-border px-3 py-2 text-[length:var(--text-sm)] text-text-primary placeholder:text-text-muted outline-none focus:border-border-focus focus:ring-2 focus:ring-ring font-mono"
                    />
                    <CustomSelect
                      value={customMinTls}
                      onChange={setCustomMinTls}
                      options={[
                        { value: "1.0", label: "TLS 1.0" },
                        { value: "1.1", label: "TLS 1.1" },
                        { value: "1.2", label: "TLS 1.2" },
                        { value: "1.3", label: "TLS 1.3" },
                      ]}
                    />
                    <button
                      type="button"
                      onClick={() => void attachCustomDomain()}
                      disabled={!customDomain.trim() || !customZoneId.trim() || saving}
                      className="h-9 px-3 rounded-lg bg-accent text-text-inverse disabled:opacity-50 flex items-center gap-2 text-[length:var(--text-xs)] font-medium"
                    >
                      <Link size={13} />
                      Attach
                    </button>
                  </div>

                  <div className="divide-y divide-border border border-border rounded-lg overflow-hidden">
                    {customDomainRows.map((domain) => (
                      <div key={String(domain.domain)} className="flex items-center justify-between gap-3 px-3 py-2 bg-bg-base">
                        <div className="min-w-0">
                          <div className="text-[length:var(--text-sm)] font-medium text-text-primary truncate">
                            {String(domain.domain ?? "")}
                          </div>
                          <div className="text-[length:var(--text-2xs)] text-text-muted truncate">
                            {String(domain.enabled ?? false)} · {JSON.stringify(domain.status ?? {})}
                          </div>
                        </div>
                        <button
                          type="button"
                          onClick={() => void deleteCustomDomain(String(domain.domain ?? ""))}
                          disabled={saving}
                          className="h-7 w-7 rounded-md text-status-error hover:bg-status-error/10 flex items-center justify-center disabled:opacity-50"
                          aria-label="Delete custom domain"
                          title="Delete custom domain"
                        >
                          <Trash2 size={13} />
                        </button>
                      </div>
                    ))}
                    {customDomainRows.length === 0 && (
                      <div className="px-3 py-5 text-center text-[length:var(--text-sm)] text-text-muted bg-bg-base">
                        No custom domains
                      </div>
                    )}
                  </div>
                </div>
              </section>
            )}

            {tab === "metrics" && (
              <section className="rounded-lg border border-border bg-bg-surface p-4">
                <div className="flex items-center justify-between gap-3 mb-3">
                  <h3 className="text-[length:var(--text-sm)] font-semibold text-text-primary">Account Metrics</h3>
                  <button
                    type="button"
                    onClick={() => void loadMetrics()}
                    disabled={!selectedConnectionId || loading}
                    className="h-8 px-3 rounded-lg border border-border text-text-secondary hover:text-text-primary hover:bg-bg-overlay disabled:opacity-50 flex items-center gap-2 text-[length:var(--text-xs)] font-medium"
                  >
                    <RefreshCw size={13} className={loading ? "animate-spin" : ""} />
                    Refresh
                  </button>
                </div>
                <pre className="max-h-[60vh] overflow-auto rounded-lg bg-bg-base border border-border p-3 text-[length:var(--text-xs)] text-text-secondary">
                  {prettyJson(metrics)}
                </pre>
              </section>
            )}
          </div>
        </main>
      </div>

      {showConnectionDialog && (
        <S3ConnectDialog onClose={() => { setShowConnectionDialog(false); void loadConnections(); }} />
      )}

      {editingConnection && (
        <S3ConnectDialog
          editConnection={editingConnection}
          onClose={() => { setEditingConnection(null); void loadConnections(); }}
        />
      )}

      {confirmAction && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-4"
          style={{ backgroundColor: "oklch(0 0 0 / 0.5)", backdropFilter: "blur(4px)" }}
          onClick={(event) => {
            if (event.target === event.currentTarget && !saving) closeConfirm();
          }}
        >
          <div className="w-full max-w-md rounded-xl bg-bg-overlay border border-border shadow-[var(--shadow-lg)]">
            <div className="px-5 pt-5 pb-4 border-b border-border">
              <h3 className="text-[length:var(--text-lg)] font-semibold text-text-primary">
                {confirmAction.title}
              </h3>
              <p className="mt-2 text-[length:var(--text-sm)] text-text-secondary">
                {confirmAction.body}
              </p>
            </div>
            <div className="px-5 py-4">
              <label className="block text-[length:var(--text-xs)] font-medium text-text-secondary mb-1">
                Type <span className="font-mono text-text-primary">{confirmAction.confirmText}</span> to confirm
              </label>
              <input
                value={confirmationText}
                onChange={(event) => setConfirmationText(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Escape" && !saving) closeConfirm();
                  if (event.key === "Enter" && confirmationMatches && !saving) {
                    event.preventDefault();
                    void confirmDestructiveAction();
                  }
                }}
                autoFocus
                className="w-full rounded-lg bg-bg-base border border-border px-3 py-2 text-[length:var(--text-sm)] text-text-primary outline-none focus:border-border-focus focus:ring-2 focus:ring-ring"
              />
            </div>
            <div className="px-5 pb-5 pt-2 flex items-center justify-end gap-2">
              <button
                type="button"
                onClick={closeConfirm}
                disabled={saving}
                className="px-4 py-2 text-[length:var(--text-sm)] text-text-secondary hover:text-text-primary rounded-lg transition-colors duration-[var(--duration-fast)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void confirmDestructiveAction()}
                disabled={!confirmationMatches || saving}
                className="px-4 py-2 text-[length:var(--text-sm)] font-medium text-text-inverse bg-status-error hover:bg-status-error/90 disabled:opacity-50 rounded-lg transition-colors duration-[var(--duration-fast)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                {confirmAction.actionLabel}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function JsonEditor({
  title,
  value,
  onChange,
  onSave,
  onDelete,
  saving,
}: {
  title: string;
  value: string;
  onChange: (value: string) => void;
  onSave: () => void;
  onDelete?: () => void;
  saving: boolean;
}) {
  return (
    <section className="rounded-lg border border-border bg-bg-surface p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-[length:var(--text-sm)] font-semibold text-text-primary">{title}</h3>
        <div className="flex items-center gap-2">
          {onDelete && (
            <button
              type="button"
              onClick={onDelete}
              disabled={saving}
              className="h-8 px-3 rounded-lg border border-status-error/40 text-status-error hover:bg-status-error/10 disabled:opacity-50 flex items-center gap-2 text-[length:var(--text-xs)] font-medium"
            >
              <Trash2 size={13} />
              Delete
            </button>
          )}
          <button
            type="button"
            onClick={onSave}
            disabled={saving}
            className="h-8 px-3 rounded-lg bg-accent text-text-inverse disabled:opacity-50 flex items-center gap-2 text-[length:var(--text-xs)] font-medium"
          >
            <Save size={13} />
            Save
          </button>
        </div>
      </div>
      <textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        spellCheck={false}
        className="min-h-[420px] w-full resize-y rounded-lg bg-bg-base border border-border p-3 font-mono text-[length:var(--text-xs)] leading-5 text-text-primary outline-none focus:border-border-focus focus:ring-2 focus:ring-ring"
      />
    </section>
  );
}
