package fr.aneo.armonik.client;

import org.junit.jupiter.api.Test;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class PemClientCertificateTest {

  private static final String VALID_CERT_PATH = "src/test/resources/camouflage/certs/client/client.cert";
  private static final String VALID_KEY_PATH = "src/test/resources/camouflage/certs/client/client.key";

  @Test
  void constructor_should_throw_exception_when_cert_path_is_null() {
    assertThatThrownBy(() -> new PemClientCertificate(null, VALID_KEY_PATH))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  void constructor_should_throw_exception_when_cert_path_is_blank() {
    assertThatThrownBy(() -> new PemClientCertificate("", VALID_KEY_PATH))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  void constructor_should_throw_exception_when_key_path_is_null() {
    assertThatThrownBy(() -> new PemClientCertificate(VALID_CERT_PATH, null))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  void constructor_should_throw_exception_when_key_path_is_blank() {
    assertThatThrownBy(() -> new PemClientCertificate(VALID_CERT_PATH, ""))
      .isInstanceOf(IllegalArgumentException.class);
  }

  @Test
  void of_should_create_instance_with_valid_paths() {
    // when
    PemClientCertificate certificate = PemClientCertificate.of(VALID_CERT_PATH, VALID_KEY_PATH);

    // then
    assertThat(certificate).isNotNull();
    assertThat(certificate.getDescription())
      .contains(VALID_CERT_PATH)
      .contains(VALID_KEY_PATH);
  }
}
