#include <iostream>
#include <mutex>

#include "logger/writer.h"

namespace API_COMMON_NAMESPACE::logger {

/**
 * @brief std::ostream -baked Writer
 */
class FileWriter : public IWriter {
private:
  std::ostream &out_;
  std::mutex mutex_;

public:
  /**
   * @brief Construct a FileWriter from a std::ostream&
   * @param out Stream to write to
   */
  FileWriter(std::ostream &out) : out_(out) {}

public:
  /**
   * @copydoc IWriter::write()
   * @details Thread-safe.
   */
  void write(Level, std::string_view message) override {
    // Lock the writer to ensure the message is written all-at-once
    std::lock_guard<std::mutex> lock_guard{mutex_};
    out_ << message << std::endl;
  }
};

/**
 * @brief Console based Writer
 */
class ConsoleWriter : public IWriter {
private:
  std::mutex mutex_;

public:
  /**
   * @copydoc IWriter::write()
   * @details Thread-safe.
   */
  void write(Level level, std::string_view message) override {
    // Lock the writer to ensure the message is written all-at-once
    std::lock_guard<std::mutex> lock_guard{mutex_};
    (level < Level::Warning ? std::cout : std::cerr) << message << std::endl;
  }
};

std::unique_ptr<IWriter> writer_console() { return std::make_unique<ConsoleWriter>(); }
std::unique_ptr<IWriter> writer_file(std::ostream &out) { return std::make_unique<FileWriter>(out); }
} // namespace API_COMMON_NAMESPACE::logger
