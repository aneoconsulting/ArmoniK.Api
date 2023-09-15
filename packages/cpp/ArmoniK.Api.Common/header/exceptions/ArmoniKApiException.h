#ifndef ARMONIK_API_ARMONIKAPIEXCEPTION_H
#define ARMONIK_API_ARMONIKAPIEXCEPTION_H

#include <stdexcept>
namespace armonik {
namespace api {
namespace common {
namespace exceptions {

class ArmoniKApiException : public std::runtime_error {
public:
  explicit ArmoniKApiException(const std::string &message) : runtime_error(message) {}
};
} // namespace exceptions
} // namespace common
} // namespace api
} // namespace armonik

#endif // ARMONIK_API_ARMONIKAPIEXCEPTION_H
