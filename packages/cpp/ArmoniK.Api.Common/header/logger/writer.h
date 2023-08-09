#pragma once
/**
 * @file writer.h
 * @brief Writter interface.
 */

#include <iosfwd>
#include <memory>
#include <string_view>

#include "level.h"

namespace API_COMMON_NAMESPACE::logger {
/**
 * @interface IWriter
 * @brief Writer interface to use by a logger.
 */
class IWriter {
public:
  /**
   * @brief Destructor.
   */
  virtual ~IWriter();

  /**
   * @brief Write a formatted message to the log.
   * @param level Log level to use for this message.
   * @param formatted formatted message to write.
   */
  virtual void write(Level level, std::string_view formatted) = 0;
};

/**
 * @brief Get a Writer to the console.
 * @return Pointer to the writer.
 */
std::unique_ptr<IWriter> writer_console();
/**
 * @brief Get a Writer to a std::ostream
 * @param out Stream to write to.
 * @return Pointer to the writer.
 */
std::unique_ptr<IWriter> writer_file(std::ostream &out);
} // namespace API_COMMON_NAMESPACE::logger
