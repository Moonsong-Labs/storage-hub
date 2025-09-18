/**
 * Mock JWT generator that matches the backend's generate_mock_jwt function
 */
export function generateMockJWT(address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"): string {
  // Header: {"alg":"HS256","typ":"JWT"} already encoded
  const header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

  // Create a mock payload with proper structure
  const payload = {
    // Standard JWT claims
    address,
    exp: 9999999999, // Expiration: far into the future for mock
    iat: 1704067200 // Issued at: 2024-01-01
  };

  // Encode payload using base64url (no padding)
  const payloadJson = JSON.stringify(payload);
  const payloadB64 = Buffer.from(payloadJson).toString("base64url");

  // Mock signature (base64url encoded)
  const signature = Buffer.from("mock_signature").toString("base64url");

  return `${header}.${payloadB64}.${signature}`;
}
