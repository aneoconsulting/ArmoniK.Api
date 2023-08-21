#ifndef ARMONIK_API_ARMONIKAPIEXCEPTION_H
#define ARMONIK_API_ARMONIKAPIEXCEPTION_H

#include <stdexcept>
namespace API_COMMON_NAMESPACE::exceptions {

class ArmoniKApiException : public std::runtime_error {
public:
  explicit ArmoniKApiException(const std::string &message) : runtime_error(message) {}
};
} // namespace API_COMMON_NAMESPACE::exceptions

#endif // ARMONIK_API_ARMONIKAPIEXCEPTION_H
