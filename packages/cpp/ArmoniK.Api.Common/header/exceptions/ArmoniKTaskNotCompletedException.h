#ifndef ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H
#define ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H

#include "ArmoniKApiException.h"
namespace armonik::api::common::exceptions {

class ArmoniKTaskNotCompletedException : public ArmoniKApiException {
public:
  explicit ArmoniKTaskNotCompletedException(const std::string &taskId, const std::string &message = "")
      : ArmoniKApiException("Task " + taskId + " not completed. " + message), taskId(taskId) {}
  const std::string taskId;
};
} // namespace armonik::api::common::exceptions

#endif // ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H
