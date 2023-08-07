#pragma once
/**
 * @file writer.h
 */

#include <iosfwd>
#include <memory>
#include <string_view>

namespace API_COMMON_NAMESPACE::logger {

class IWriter {
public:
  virtual ~IWriter();

  virtual void write(std::string_view formatted);
};

std::unique_ptr<IWriter> writer_file(std::ostream &out);
} // namespace API_COMMON_NAMESPACE::logger
