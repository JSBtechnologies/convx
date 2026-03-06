export type LicenseStatusKind =
  | 'valid'
  | 'grace_period'
  | 'expired'
  | 'revoked'
  | 'tampered'
  | 'device_mismatch'
  | 'not_activated'
  | 'error';

export interface LicenseStatus {
  status: LicenseStatusKind;
  device_name?: string;
  recheck_after?: string;
  days_overdue?: number;
  stored_device?: string;
  error_message?: string;
}

export interface LicenseInfo {
  key_masked: string;
  device_name: string;
  platform: string;
  activated_at: string;
  recheck_after: string;
}

export type ActivateResult =
  | { outcome: 'activated'; device_name: string }
  | { outcome: 'already_active'; device_name: string }
  | { outcome: 'no_config' }
  | { outcome: 'error'; message: string };

export interface EnterpriseConfig {
  has_config: boolean;
  org_id?: string;
  settings?: EnterpriseSettings;
}

export interface EnterpriseSettings {
  default_quality?: number;
  default_format?: string;
  output_directory?: string;
  overwrite_existing?: boolean;
  show_notifications?: boolean;
  allowed_formats?: string[];
  locked: boolean;
}
