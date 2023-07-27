#ifndef ARMONIK_API_PROCESSSTATUS_H
#define ARMONIK_API_PROCESSSTATUS_H

#include <string>
#include <utility>

namespace API_WORKER_NAMESPACE {
class ProcessStatus {
public:
  ProcessStatus() : ProcessStatus(true, "") {}
  explicit ProcessStatus(const char *error_message) : ProcessStatus(false, std::string(error_message)) {}
  explicit ProcessStatus(std::string error_message) : ProcessStatus(false, std::move(error_message)) {}

  [[nodiscard]] bool ok() const { return ok_; }
  [[nodiscard]] const std::string &details() const { return details_; }
  void set_ok() {
    ok_ = true;
    details_.clear();
  }
  void set_error(const std::string &details) {
    ok_ = false;
    details_ = details;
  }

  static const ProcessStatus Ok;
  static const ProcessStatus Error;

private:
  explicit ProcessStatus(bool ok, std::string error_message = "") {
    ok_ = ok;
    details_ = std::move(error_message);
  }
  bool ok_ = true;
  std::string details_;
};
} // namespace API_WORKER_NAMESPACE

#endif // ARMONIK_API_PROCESSSTATUS_H
