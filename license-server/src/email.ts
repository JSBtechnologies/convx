import type { Env } from './types';

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export async function sendLicenseEmail(
  env: Env,
  to: string,
  name: string | null,
  licenseKey: string,
  discountCode: string,
): Promise<void> {
  const displayName = name || 'there';
  const html = buildHtml(displayName, licenseKey, discountCode);
  const text = buildText(displayName, licenseKey, discountCode);

  const response = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${env.RESEND_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      from: env.EMAIL_FROM,
      reply_to: 'license@convx.dev',
      to: [to],
      subject: 'Your ConvX License Key',
      html,
      text,
    }),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`Resend API error ${response.status}: ${errorBody}`);
  }
}

function buildHtml(name: string, licenseKey: string, discountCode: string): string {
  const safeName = escapeHtml(name);
  const safeKey = escapeHtml(licenseKey);
  const safeDiscount = escapeHtml(discountCode);
  return `<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; color: #333; background: #fafafa;">
  <div style="background: #fff; border-radius: 12px; padding: 32px; border: 1px solid #e5e5e5;">
    <h1 style="color: #1a1a2e; margin-top: 0;">Welcome to ConvX!</h1>
    <p>Hey ${safeName},</p>
    <p>Thanks for purchasing ConvX. Here's your license key:</p>
    <div style="background: #f5f5f5; border: 2px solid #1a1a2e; border-radius: 8px; padding: 20px; text-align: center; margin: 24px 0;">
      <code style="font-size: 22px; font-weight: bold; letter-spacing: 2px; color: #1a1a2e;">${safeKey}</code>
    </div>
    <p><strong>Download ConvX:</strong></p>
    <div style="text-align: center; margin: 24px 0;">
      <a href="https://convx.dev/download" style="display: inline-block; padding: 12px 28px; background: #1a1a2e; color: #fff; text-decoration: none; border-radius: 8px; font-weight: 600; font-size: 16px;">Download ConvX</a>
    </div>
    <p style="font-size: 14px; color: #666;">Enter your license key on the download page to get the installer for your platform.</p>

    <p><strong>To activate after installing:</strong></p>
    <ol>
      <li>Open ConvX</li>
      <li>Enter your license key when prompted</li>
      <li>Or run: <code style="background: #f5f5f5; padding: 2px 6px; border-radius: 4px;">convx activate ${safeKey}</code></li>
    </ol>

    <hr style="border: none; border-top: 1px solid #e5e5e5; margin: 32px 0;" />

    <h2 style="color: #1a1a2e;">Bonus: $14 off Agent Toolbox</h2>
    <p>As a ConvX owner, you get <strong>Agent Toolbox for $15</strong> instead of $29. Manage, sandbox, and secure all your AI tools in one place.</p>
    <div style="background: #f0fdf4; border: 2px solid #16a34a; border-radius: 8px; padding: 20px; text-align: center; margin: 24px 0;">
      <code style="font-size: 20px; font-weight: bold; letter-spacing: 2px; color: #16a34a;">${safeDiscount}</code>
    </div>
    <p>Use this code at <a href="https://getagenttoolbox.com" style="color: #6366f1;">getagenttoolbox.com</a> checkout. Single-use, valid for 1 year.</p>
  </div>
  <p style="color: #999; font-size: 12px; text-align: center; margin-top: 24px;">ConvX &mdash; <a href="https://convx.dev" style="color: #999;">convx.dev</a></p>
</body>
</html>`;
}

export async function sendEnterpriseWelcomeEmail(
  env: Env,
  to: string,
  name: string | null,
  plan: string,
  seats: number,
  sampleKeys: string[],
  dashboardUrl: string,
): Promise<void> {
  const rawDisplayName = name || 'there';
  const rawPlanCapitalized = plan.charAt(0).toUpperCase() + plan.slice(1);
  const displayName = escapeHtml(rawDisplayName);
  const planCapitalized = escapeHtml(rawPlanCapitalized);
  const keysDisplay = sampleKeys.map((k) => `  ${k}`).join('\n');
  const safeDashboardUrl = escapeHtml(dashboardUrl);

  const html = `<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; color: #333; background: #fafafa;">
  <div style="background: #fff; border-radius: 12px; padding: 32px; border: 1px solid #e5e5e5;">
    <h1 style="color: #1a1a2e; margin-top: 0;">Welcome to ConvX Enterprise!</h1>
    <p>Hey ${displayName},</p>
    <p>Your <strong>ConvX ${planCapitalized}</strong> plan is ready. Here's what you need to get started:</p>

    <h2 style="color: #1a1a2e;">1. Set Up Your Admin Dashboard</h2>
    <p>Create your account to manage licenses, seats, and settings:</p>
    <div style="text-align: center; margin: 24px 0;">
      <a href="${safeDashboardUrl}" style="display: inline-block; padding: 14px 32px; background: #6366f1; color: #fff; text-decoration: none; border-radius: 8px; font-weight: 600; font-size: 16px;">Set Up Dashboard</a>
    </div>

    <h2 style="color: #1a1a2e;">2. Your License Keys</h2>
    <p>You have <strong>${seats} seats</strong>. Here are your first keys:</p>
    <div style="background: #f5f5f5; border: 2px solid #1a1a2e; border-radius: 8px; padding: 16px; margin: 16px 0; font-family: monospace; font-size: 13px; line-height: 2;">
      ${sampleKeys.map((k) => `<code>${escapeHtml(k)}</code><br>`).join('')}
      ${seats > 5 ? `<span style="color: #666;">...and ${seats - 5} more (see dashboard)</span>` : ''}
    </div>

    <h2 style="color: #1a1a2e;">3. Deploy to Your Team</h2>
    <p>Three ways to distribute ConvX:</p>
    <ul>
      <li><strong>Manual:</strong> Share license keys with team members</li>
      <li><strong>Silent activation:</strong> Deploy <code>enterprise-config.json</code> with a pre-set key</li>
      <li><strong>MDM:</strong> Push config via Jamf, Intune, or GPO — zero user friction</li>
    </ul>

    <p>Full MDM deployment guide: <a href="https://docs.convx.dev/enterprise/mdm" style="color: #6366f1;">docs.convx.dev/enterprise/mdm</a></p>

    <hr style="border: none; border-top: 1px solid #e5e5e5; margin: 32px 0;" />
    <p style="font-size: 14px; color: #666;">Questions? Reply to this email or contact <a href="mailto:enterprise@convx.dev" style="color: #6366f1;">enterprise@convx.dev</a></p>
  </div>
  <p style="color: #999; font-size: 12px; text-align: center; margin-top: 24px;">ConvX Enterprise &mdash; <a href="https://enterprise.convx.dev" style="color: #999;">enterprise.convx.dev</a></p>
</body>
</html>`;

  const text = `Welcome to ConvX Enterprise!

Hey ${rawDisplayName},

Your ConvX ${rawPlanCapitalized} plan is ready.

1. SET UP YOUR ADMIN DASHBOARD
${dashboardUrl}

2. YOUR LICENSE KEYS (${seats} seats)
${keysDisplay}
${seats > 5 ? `...and ${seats - 5} more (see dashboard)` : ''}

3. DEPLOY TO YOUR TEAM
- Manual: Share keys with team members
- Silent: Deploy enterprise-config.json with pre-set key
- MDM: Push via Jamf, Intune, or GPO

MDM guide: https://docs.convx.dev/enterprise/mdm

Questions? Contact enterprise@convx.dev

--
ConvX Enterprise - https://enterprise.convx.dev`;

  const response = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${env.RESEND_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      from: env.EMAIL_FROM,
      reply_to: 'enterprise@convx.dev',
      to: [to],
      subject: `Your ConvX ${rawPlanCapitalized} Plan is Ready`,
      html,
      text,
    }),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`Resend API error ${response.status}: ${errorBody}`);
  }
}

function buildText(name: string, licenseKey: string, discountCode: string): string {
  return `Welcome to ConvX!

Hey ${name},

Thanks for purchasing ConvX. Here's your license key:

  ${licenseKey}

Download ConvX: https://convx.dev/download
Enter your license key on the download page to get the installer.

To activate after installing:
1. Open ConvX
2. Enter your license key when prompted
3. Or run: convx activate ${licenseKey}

---

BONUS: $14 off Agent Toolbox

As a ConvX owner, you get Agent Toolbox for $15 instead of $29.
Manage, sandbox, and secure all your AI tools in one place.

Your discount code: ${discountCode}

Use it at https://getagenttoolbox.com checkout.
Single-use, valid for 1 year.

--
ConvX - https://convx.dev`;
}
