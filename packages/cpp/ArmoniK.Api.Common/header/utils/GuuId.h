/**
 * @file GuuId.h
 * @brief Header file for the GuuId class, providing a UUID generation method.
 */

#pragma once

#include <iomanip>
#include <iostream>
#include <random>

/**
 * @brief The armonik::api::common::utils namespace provides utility classes and functions for the ArmoniK API.
 */
namespace armonik {
namespace api {
namespace common {
namespace utils {
/**
 * @class GuuId
 * @brief The GuuId class provides a static method for generating UUIDs.
 */
class GuuId {
public:
  /**
   * @brief Generates a random UUID string.
   *
   * This method generates a random UUID string, following the format "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx".
   *
   * @return A std::string containing the generated UUID.
   */
  static std::string generate_uuid() {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dis(0, 15);

    const std::string chars = "0123456789abcdef";
    std::string uuid = "";

    for (int i = 0; i < 32; i++) {
      uuid += chars[dis(gen)];
      if (i == 7 || i == 11 || i == 15 || i == 19) {
        uuid += '-';
      }
    }

    return uuid;
  }
};
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
