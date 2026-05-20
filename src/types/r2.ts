export interface R2Bucket {
  name: string | null;
  creation_date: string | null;
  jurisdiction: string | null;
  location: string | null;
  storage_class: string | null;
}

export interface R2CreateBucketRequest {
  name: string;
  jurisdiction?: string | null;
  locationHint?: string | null;
  storageClass?: string | null;
}

export interface R2PatchBucketRequest {
  storageClass?: string | null;
}

export interface R2AttachCustomDomainRequest {
  domain: string;
  zoneId: string;
  enabled?: boolean;
  minTls?: string | null;
}

export type R2Json = Record<string, unknown> | unknown[] | string | number | boolean | null;
