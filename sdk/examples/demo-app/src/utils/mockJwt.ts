/**
 * Convert base64 to base64url format
 * base64url is base64 with URL-safe characters (+ becomes -, / becomes _) and no padding (=)
 */
function base64ToBase64url(base64: string): string {
  return base64
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

/**
 * Encode string to base64url format
 */
function encodeBase64url(str: string): string {
  // For browser compatibility, use btoa if available, otherwise use Buffer
  if (typeof btoa !== 'undefined') {
    return base64ToBase64url(btoa(str));
  }
  return base64ToBase64url(Buffer.from(str).toString('base64'));
}

/**
 * Mock JWT generator that matches the backend's generate_mock_jwt function
 * Used for testing and development purposes to bypass SIWE authentication
 */
export function generateMockJWT(address: string): string {
  // Header: {"alg":"HS256","typ":"JWT"} already encoded
  const header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

  // Create a mock payload with proper structure
  const payload = {
    address,
    // Standard JWT claims
    sub: address, // Subject: user's ETH address
    exp: 9999999999, // Expiration: far into the future for mock
    iat: 1704067200 // Issued at: 2024-01-01
  };

  // Encode payload using base64url (no padding)
  const payloadJson = JSON.stringify(payload);
  const payloadB64 = encodeBase64url(payloadJson);

  // Mock signature (base64url encoded)
  const signature = encodeBase64url("mock_signature");

  return `${header}.${payloadB64}.${signature}`;
}
