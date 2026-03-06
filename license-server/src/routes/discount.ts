import { findDiscountByCode, redeemDiscountCode } from '../db';
import type { RedeemDiscountRequest, Supabase, ValidateDiscountRequest, ValidateDiscountResponse } from '../types';

const TOOLBOX_PRICE_CENTS = 2900; // $29

export async function handleValidateDiscount(
  body: ValidateDiscountRequest,
  supabase: Supabase,
): Promise<Response> {
  const { code } = body;
  if (!code) {
    return Response.json({ error: 'Missing code' }, { status: 400 });
  }

  const normalized = code.trim().toUpperCase();
  const discount = await findDiscountByCode(supabase, normalized);

  if (!discount) {
    return Response.json({ valid: false, error: 'Invalid discount code' }, { status: 404 });
  }

  if (discount.redeemed) {
    return Response.json({ valid: false, error: 'Code already redeemed' }, { status: 410 });
  }

  if (discount.expires_at && new Date(discount.expires_at) < new Date()) {
    return Response.json({ valid: false, error: 'Code expired' }, { status: 410 });
  }

  const finalPrice = TOOLBOX_PRICE_CENTS - discount.discount_cents;
  const response: ValidateDiscountResponse = {
    valid: true,
    discount_cents: discount.discount_cents,
    discount_formatted: `$${(discount.discount_cents / 100).toFixed(2)}`,
    original_price_cents: TOOLBOX_PRICE_CENTS,
    final_price_cents: finalPrice,
    product: 'Agent Toolbox',
  };

  return Response.json(response);
}

export async function handleRedeemDiscount(
  body: RedeemDiscountRequest,
  supabase: Supabase,
): Promise<Response> {
  const { code, email } = body;
  if (!code || !email) {
    return Response.json({ error: 'Missing code or email' }, { status: 400 });
  }

  const normalized = code.trim().toUpperCase();
  const discount = await findDiscountByCode(supabase, normalized);

  if (!discount) {
    return Response.json({ error: 'Invalid discount code' }, { status: 404 });
  }

  if (discount.redeemed) {
    return Response.json({ error: 'Code already redeemed' }, { status: 410 });
  }

  if (discount.expires_at && new Date(discount.expires_at) < new Date()) {
    return Response.json({ error: 'Code expired' }, { status: 410 });
  }

  const redeemed = await redeemDiscountCode(supabase, normalized, email);
  if (!redeemed) {
    return Response.json({ error: 'Code already redeemed' }, { status: 410 });
  }

  return Response.json({
    redeemed: true,
    discount_cents: discount.discount_cents,
    final_price_cents: TOOLBOX_PRICE_CENTS - discount.discount_cents,
  });
}
