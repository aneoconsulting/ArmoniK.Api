#ifndef ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H
#define ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H

#include "ArmoniKApiException.h"
namespace API_COMMON_NAMESPACE::exceptions {

class ArmoniKTaskNotCompletedException : public ArmoniKApiException {
public:
  explicit ArmoniKTaskNotCompletedException(const std::string &taskId, const std::string &message = "")
      : ArmoniKApiException("Task " + taskId + " not completed. " + message), taskId(taskId) {}
  const std::string taskId;
};
} // namespace API_COMMON_NAMESPACE::exceptions

#endif // ARMONIK_API_ARMONIKTASKNOTCOMPLETEDEXCEPTION_H
