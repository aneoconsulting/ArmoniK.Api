#ifndef ARMONIK_API_ARMONIKAPIEXCEPTION_H
#define ARMONIK_API_ARMONIKAPIEXCEPTION_H

#include <stdexcept>
namespace armonik::api::common::exceptions {

class ArmoniKApiException : public std::runtime_error {
public:
  explicit ArmoniKApiException(const std::string &message) : runtime_error(message) {}
};
} // namespace armonik::api::common::exceptions

#endif // ARMONIK_API_ARMONIKAPIEXCEPTION_H
