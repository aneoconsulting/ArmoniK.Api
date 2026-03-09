/*
 * Copyright © 2025 ANEO (armonik@aneo.fr)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package fr.aneo.armonik.client;

import java.time.Duration;


/**
 * Retry policy configuration for gRPC channels with exponential backoff.
 *
 * <p>Configures automatic retry behavior for transient failures on gRPC calls.
 * Retries are attempted only for specific status codes that indicate transient failures:
 * <ul>
 *   <li><strong>UNAVAILABLE</strong> - Service temporarily unavailable</li>
 *   <li><strong>DEADLINE_EXCEEDED</strong> - Request timeout</li>
 *   <li><strong>RESOURCE_EXHAUSTED</strong> - Rate limiting or quota exceeded</li>
 *   <li><strong>ABORTED</strong> - Operation aborted (e.g., transaction conflicts)</li>
 * </ul>
 *
 * <p>Non-retryable errors (e.g., INVALID_ARGUMENT, NOT_FOUND, PERMISSION_DENIED, UNAUTHENTICATED)
 * will fail immediately without retry attempts.
 *
 * <h3>Exponential Backoff Example</h3>
 * With default settings (initialBackoff=1s, multiplier=1.5, maxBackoff=5s):
 * <pre>
 * Attempt 1 fails → wait 1.0s
 * Attempt 2 fails → wait 1.5s (1.0s × 1.5)
 * Attempt 3 fails → wait 2.25s (1.5s × 1.5)
 * Attempt 4 fails → wait 3.375s (2.25s × 1.5)
 * Attempt 5 fails → wait 5s (capped at maxBackoff)
 * </pre>
 *
 * @param maxAttempts Total number of attempts (1 initial + retries). Must be ≥ 1.
 * @param initialBackoff Initial delay before first retry. Must be positive.
 * @param maxBackoff Maximum delay cap to prevent unbounded exponential growth. Must be positive and ≥ initialBackoff.
 * @param backoffMultiplier Multiplier for exponential backoff growth. Must be > 0. Common values: 1.5-2.0.
 */

public record RetryPolicy(
  int maxAttempts,
  Duration initialBackoff,
  Duration maxBackoff,
  double backoffMultiplier
) {

  /**
   * Default retry policy
   * <ul>
   *   <li>maxAttempts: 5 (1 initial + 4 retries)</li>
   *   <li>initialBackoff: 1 second</li>
   *   <li>maxBackoff: 5 seconds</li>
   *   <li>backoffMultiplier: 1.5 (exponential growth)</li>
   * </ul>
   */
  public static RetryPolicy DEFAULT = new RetryPolicy(5, Duration.ofSeconds(1), Duration.ofSeconds(5), 1.5);

  /**
   * Compact constructor with validation.
   *
   * @throws IllegalArgumentException if any parameter is invalid
   */
  public RetryPolicy {
    if (maxAttempts < 1) {
      throw new IllegalArgumentException("maxAttempts must be >= 1, got: " + maxAttempts);
    }
    if (initialBackoff == null || initialBackoff.isNegative() || initialBackoff.isZero()) {
      throw new IllegalArgumentException("initialBackoff must be positive, got: " + initialBackoff);
    }
    if (maxBackoff == null || maxBackoff.isNegative() || maxBackoff.isZero()) {
      throw new IllegalArgumentException("maxBackoff must be positive, got: " + maxBackoff);
    }
    if (maxBackoff.compareTo(initialBackoff) < 0) {
      throw new IllegalArgumentException(
        String.format("maxBackoff (%s) must be >= initialBackoff (%s)", maxBackoff, initialBackoff)
      );
    }
    if (backoffMultiplier <= 0) {
      throw new IllegalArgumentException("backoffMultiplier must be > 0, got: " + backoffMultiplier);
    }
  }
}
