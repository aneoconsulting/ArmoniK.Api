#include <iostream>
#include <mutex>

#include "logger/writer.h"

namespace API_COMMON_NAMESPACE::logger {

class FileWriter : public IWriter {
private:
  std::ostream &out_;
  std::mutex mutex_;

public:
  FileWriter(std::ostream &out) : out_(out) {}

public:
  void write(Level, std::string_view message) override {
    std::lock_guard<std::mutex> lock_guard{mutex_};
    out_ << message << std::endl;
  }
};

class ConsoleWriter : public IWriter {
private:
  std::mutex mutex_;

public:
  void write(Level level, std::string_view message) override {
    std::lock_guard<std::mutex> lock_guard{mutex_};
    (level < Level::Warning ? std::cout : std::cerr) << message << std::endl;
  }
};

std::unique_ptr<IWriter> writer_console() { return std::make_unique<ConsoleWriter>(); }
std::unique_ptr<IWriter> writer_file(std::ostream &out) { return std::make_unique<FileWriter>(out); }
} // namespace API_COMMON_NAMESPACE::logger
