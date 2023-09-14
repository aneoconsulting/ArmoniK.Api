#ifndef ARMONIK_API_ARMONIKTASKERROR_H
#define ARMONIK_API_ARMONIKTASKERROR_H

#include "ArmoniKApiException.h"
#include <objects.pb.h>
#include <sstream>
#include <vector>
namespace armonik {
namespace api {
namespace common {
namespace exceptions {

class ArmoniKTaskError : public ArmoniKApiException {
public:
  explicit ArmoniKTaskError(const std::string &message, const armonik::api::grpc::v1::TaskError &task_error)
      : ArmoniKApiException(message) {
    std::stringstream ss;
    ss << "TaskId : " << task_error.task_id() << " Errors : ";
    for (auto &&e : task_error.errors()) {
      std::string status = armonik::api::grpc::v1::task_status::TaskStatus_Name(e.task_status());
      status_details.emplace_back(status, e.detail());
      ss << '\n' << status << " : " << e.detail();
    }
    details = std::string(ArmoniKApiException::what()) + " : " + ss.str();
    taskId_ = task_error.task_id();
  }
  [[nodiscard]] const char *what() const noexcept override { return details.c_str(); }
  const std::string &taskId() { return taskId_; }
  const std::vector<std::pair<std::string, std::string>> &error_details() { return status_details; }

private:
  std::string details;
  std::string taskId_;
  std::vector<std::pair<std::string, std::string>> status_details;
};

} // namespace exceptions
} // namespace common
} // namespace api
} // namespace armonik

#endif // ARMONIK_API_ARMONIKTASKERROR_H
