/**

@file serilog_log_entry.h
@brief Defines a class that represents a log entry in the Serilog format.
*/
#pragma once

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#include "serilog/SerilogContext.h"
#include "serilog/serilog_typedef.h"
#include "utils/StringsUtils.h"

namespace API_COMMON_NAMESPACE::serilog
/**

@brief Increments the logging level.
@param other The logging level to be incremented.
@return The incremented logging level.
*/
{
inline logging_level &operator++(logging_level &other) {
  other = static_cast<logging_level>(std::min((other + 1), 5));
  return other;
}

/**

@brief Increments the logging level.
@param other The logging level to be incremented.
@param dummy A dummy integer parameter to indicate postfix increment.
@return The original logging level value.
*/
inline logging_level operator++(logging_level &other, int) {
  const logging_level r_val = other;
  ++other;
  return r_val;
}

/**

@brief Decrements the logging level.
@param other The logging level to be decremented.
@return The decremented logging level.
*/
inline logging_level &operator--(logging_level &other) {
  other = static_cast<logging_level>(std::max((other - 1), 0));
  return other;
}

/**

@brief Decrements the logging level.
@param other The logging level to be decremented.
@param dummy A dummy integer parameter to indicate postfix decrement.
@return The original logging level value.
*/
inline logging_level operator--(logging_level &other, int) {
  const logging_level r_val = other;
  --other;
  return r_val;
}

/**

@brief String representation of the logging levels.
*/
const std::string logging_level_strings[6] = {"Verbose", "Debug", "Information", "Warning", "Error", "Fatal"};

/**
 * @brief Short string representation of the logging levels.
 *
 */
const std::string logging_level_strings_short[6] = {"VRB", "DBG", "INF", "WRN", "ERR", "FTL"};

/**
 * @brief Represents a log entry in the Serilog format.
 *
 */
class serilog_log_entry;

/**
 * @brief Represents a log entry in the Serilog format.
 *
 *
 */
class serilog_log_entry {
public:
  /*
   * @brief Constructs a new serilog_log_entry object.
   * @param message The log message.
   * @param context The Serilog context of the log entry.
   *
   */
  serilog_log_entry(std::string message, serilog_context &&context)
      : context(std::move(context)), _message(std::move(message)) {
    init_time();
  }

  /**
   * @brief Returns the log entry as a raw JSON string.
   * @return The log entry as a raw JSON string.
   *
   */
  [[nodiscard]] std::string to_raw_json_entry() const {
    std::stringstream string_stream;
    string_stream << R"({"@t": ")" << time << R"(", "@mt":")" << utils::string_tools::escape_json(_message)
                  << R"(", "@l":")" << logging_level_strings[context.level] << std::string(R"(","Logger":")")
                  << context.logger_name << std::string("\"");
    if (context.empty()) {
      string_stream << "}";
      return string_stream.str();
    }
    const auto parameters_specified = context.size();
    for (size_t i = 0; i < parameters_specified; ++i) {
      string_stream << ",\"" << utils::string_tools::escape_json(context[i].first) << "\":\""
                    << utils::string_tools::escape_json(context[i].second.str_val) << "\"";
    }
    string_stream << "}";
    const auto str = string_stream.str();
    const auto cc = str.c_str();
    return cc;
  }

  /**
   * @brief Returns the log message.
   * @return The log message.
   *
   */
  [[nodiscard]] const std::string &message() const { return _message; }

  const serilog_context context;
  char time[24]{};

private:
  /**
   * @brief Initializes the time of the log entry.
   */
  void init_time() {
    auto str = std::to_string(
        std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
            .count() %
        1000);

    std::stringstream ss;
    ss << std::setw(3) << std::setfill('0') << str;
    str = ss.str();
    time_t raw_time;
    std::time(&raw_time);
    const struct tm *time_info = localtime(&raw_time);
    strftime(time, 23, "%FT%T.", time_info);
    time[19] = '.';
    std::copy(str.data(), &str[3], &time[20]);
    time[23] = '\0';
  }

  /**
   * @brief The log message.
   *
   */
  std::string _message;
};

/**
 @class serilog
 @brief A class for logging messages with different levels and properties using Serilog format.
*/
class serilog {
public:
  inline static logging_level base_level;
  inline static logging_level base_level_serilog;

  logging_level level = logging_level::debug;
  logging_level level_serilog = logging_level::verbose;

  logging_format format_mode = CONSOLE;

  /**
    @brief Default constructor.
  */
  serilog(logging_format l_format_mode = CONSOLE) : format_mode(l_format_mode) { initialize(name_); }

  /**
    @brief Constructor that takes a name and a vector of properties.
    @param name The name of the logger.
    @param properties A vector of properties.
  */
  serilog(const char *name, serilog_properties_vector_t &&properties, logging_format l_format_mode = CONSOLE)
      : format_mode(l_format_mode), properties_(std::move(properties)) {
    initialize(name);
  }

  /**
    @brief Constructor that takes a name and a single property pair.
    @param name The name of the logger.
    @param property_pair A single property pair.
  */
  serilog(const char *name, serilog_properties_pair_t &&property_pair, logging_format l_format_mode = CONSOLE)
      : format_mode(l_format_mode), properties_({std::move(property_pair)}) {
    initialize(name);
  }

  /**
    @brief Constructor that takes a name.
    @param name The name of the logger.
  */
  explicit serilog(const char *name, logging_format l_format_mode = CONSOLE) : format_mode(l_format_mode) {
    initialize(name);
  }

  /**
    @brief Constructor that takes a name and logging levels for console and Serilog.
    @param name The name of the logger.
    @param console_logging_level The logging level for console output.
    @param serilog_logging_level The logging level for Serilog output.
  */
  explicit serilog(const char *name, const logging_level console_logging_level,
                   const logging_level serilog_logging_level, logging_format l_format_mode = CONSOLE)
      : format_mode(l_format_mode) {
    initialize(name);
    level = console_logging_level;
    level_serilog = serilog_logging_level;
  }

  /**
    @brief Destructor.
  */
  ~serilog() {
    enrich_.clear();
    if (!static_instance_) {
      // std::cout << "Remove last dynamic object" << std::endl;
      std::lock_guard<std::mutex> guard(logs_mutex_);
      // std::cout << "Get logs_mutex_ OK" << std::endl;
      unregister_logger(this);

      // std::cout << "unregister logger OK" << std::endl;
      shared_instance().transfer_logs(serilog_dispatch_queue_);
      shared_instance().format_mode = format_mode;
      // std::cout << "transfer logger OK" << std::endl;
      return;
    }
    s_terminating_ = true;
    std::unique_lock<std::mutex> lock{s_thread_finished_mutex_};
    s_thread_finished_.wait(lock);
    // std::cout << std::string{"Destroy serilog"} << std::endl;
  }

  serilog(const serilog &) = delete;

  serilog(serilog &&) = delete;

  serilog &operator=(const serilog &) = delete;

  serilog &operator=(serilog &&) = delete;

  /**
    @brief Sends events to the handler.
  */
  static void send_events_handler() {
    bool has_data(false);
    std::stringstream string_stream;
    {
      std::lock_guard<std::mutex> static_guard(s_loggers_mutex_);
      if (_s_loggers.empty())
        return;
      int64_t index = static_cast<int64_t>(_s_loggers.size()) - 1;

      while (index >= 0) {
        const auto &logger = _s_loggers[index];
        std::lock_guard<std::mutex> guard(logger->logs_mutex_);

        while (!logger->serilog_dispatch_queue_.empty()) {
          // std::cout << "new message : " << logger->format_mode << std::endl;
          if (logger->format_mode == SEQ) {
            has_data = true;
            string_stream << logger->serilog_dispatch_queue_.back()->to_raw_json_entry() << "\n";
          }

          delete logger->serilog_dispatch_queue_.back();
          logger->serilog_dispatch_queue_.pop_back();
        }

        --index;
      }
    }
    if (has_data) {
      try {
        std::cout << string_stream.str();
      } catch (const std::exception &e) {
        log_error("Error while trying to ingest logs:", {{"What", utils::json_string{e.what()}}});
        throw;
      }
    }
  }

private:
  static void init(const logging_level verbosity, const logging_level serilog_verbosity,
                   const size_t dispatch_interval = 1000) {
    if (s_initialized_)
      return;

    s_initialized_ = true;
    base_level = verbosity;
    base_level_serilog = serilog_verbosity;
    _s_dispatch_interval = std::chrono::milliseconds(dispatch_interval);
    // std::cout << "start serilog thread" << std::endl;
    shared_instance().start_thread();
  }

public:
  void add_property(std::string key, utils::json_string val) {
    properties_.emplace_back(std::move(key), std::move(val));
  }

  static void add_shared_property(std::string key, utils::json_string val) {
    s_shared_properties_.emplace_back(std::move(key), std::move(val));
  }

  void enrich(std::function<void(serilog_context &)> enrich) { enrich_.push_back(std::move(enrich)); }

  static void add_shared_enrich(std::function<void(serilog_context &)> enrich) {
    s_enrich_.push_back(std::move(enrich));
  }

  /**
    @brief Log a verbose message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void verbose(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::verbose>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a debug message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void debug(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::debug>(std::move(message), std::move(properties));
  }

  /**
    @brief Log an info message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void info(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::info>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a warning message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void warning(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::warning>(std::move(message), std::move(properties));
  }

  /**
    @brief Log an error message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void error(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::error>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a fatal message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  void fatal(std::string message, serilog_properties_vector_t &&properties) const {
    instance_log_generic<logging_level::fatal>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a verbose message without properties.
    @param message The message to log.
  */
  void verbose(std::string message) const { instance_log_generic<logging_level::verbose>(std::move(message)); }

  /**
    @brief Log a debug message without properties.
    @param message The message to log.
  */
  void debug(std::string message) const { instance_log_generic<logging_level::debug>(std::move(message)); }

  /**
    @brief Log an info message without properties.
    @param message The message to log.
  */
  void info(std::string message) const { instance_log_generic<logging_level::info>(std::move(message)); }

  /**
    @brief Log a warning message without properties.
    @param message The message to log.
  */
  void warning(std::string message) const { instance_log_generic<logging_level::warning>(std::move(message)); }

  /**
    @brief Log an error message without properties.
    @param message The message to log.
  */
  void error(std::string message) const { instance_log_generic<logging_level::error>(std::move(message)); }

  /**
    @brief Log a fatal message without properties.
    @param message The message to log.
  */
  void fatal(std::string message) const { instance_log_generic<logging_level::fatal>(std::move(message)); }

  /**
    @brief Log a static verbose message.
    @param message The message to log.
  */
  static void log_verbose(std::string message) {
    shared_instance().instance_log_generic<logging_level::verbose>(std::move(message));
  }

  /**
    @brief Log a static debug message.
    @param message The message to log.
  */
  static void log_debug(std::string message) {
    shared_instance().instance_log_generic<logging_level::debug>(std::move(message));
  }

  /**
    @brief Log a static info message.
    @param message The message to log.
  */
  static void log_info(std::string message) {
    shared_instance().instance_log_generic<logging_level::info>(std::move(message));
  }

  /**
    @brief Log a static warning message.
    @param message The message to log.
  */
  static void log_warning(std::string message) {
    shared_instance().instance_log_generic<logging_level::warning>(std::move(message));
  }

  /**
    @brief Log a static error message.
    @param message The message to log.
  */
  static void log_error(std::string message) {
    shared_instance().instance_log_generic<logging_level::error>(std::move(message));
  }

  /**
    @brief Log a static fatal message.
    @param message The message to log.
  */
  static void log_fatal(std::string message) {
    shared_instance().instance_log_generic<logging_level::fatal>(std::move(message));
  }

  /**
    @brief Log a static verbose message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_verbose(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::verbose>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a static debug message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_debug(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::debug>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a static info message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_info(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::info>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a static warning message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_warning(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::warning>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a static error message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_error(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::error>(std::move(message), std::move(properties));
  }

  /**
    @brief Log a static fatal message with properties.
    @param message The message to log.
    @param properties Additional properties for the log entry.
  */
  static void log_fatal(std::string message, serilog_properties_vector_t &&properties) {
    shared_instance().instance_log_generic<logging_level::fatal>(std::move(message), std::move(properties));
  }

  // endregion
private:
  /**
   * @brief Flag to check if the logger is initialized.
   */
  inline static bool s_initialized_;

  /**
   * @brief Flag to check if the logger is terminating.
   */
  inline static bool s_terminating_;

  /**
   * @brief Duration between log dispatches.
   */
  inline static std::chrono::duration<long long, std::milli> _s_dispatch_interval;

  /**
   * @brief Mutex for thread finished condition variable.
   */
  inline static std::mutex s_thread_finished_mutex_;

  /**
   * @brief Condition variable for thread finished.
   */
  inline static std::condition_variable s_thread_finished_;

  /**
   * @brief Mutex for loggers.
   */
  inline static std::mutex s_loggers_mutex_;

  /**
   * @brief Vector containing all registered loggers.
   */
  inline static std::vector<serilog *> _s_loggers;

  /**
   * @brief Atomic integer to store the logger ID.
   */
  inline static std::atomic_int32_t s_logger_id_{0};

  /**
   * @brief Shared properties for all loggers.
   */
  inline static serilog_properties_vector_t s_shared_properties_;

  /**
   * @brief Shared enrich functions for all loggers.
   */
  inline static std::vector<std::function<void(serilog_context &)>> s_enrich_;

  /**
   * @brief Log dispatch queue.
   */
  mutable std::vector<serilog_log_entry *> serilog_dispatch_queue_;

  /**
   * @brief Mutex for logs.
   */
  mutable std::mutex logs_mutex_;

  /**
   * @brief Flag to check if the logger is a static instance.
   */
  bool static_instance_{false};

  /**
   * @brief Logger name.
   */
  char name_[32]{"Default\0"};

  /**
   * @brief Logger properties.
   */
  serilog_properties_vector_t properties_;

  /**
   * @brief Enrich functions for the logger.
   */
  std::vector<std::function<void(serilog_context &)>> enrich_;

  /**
   * @brief Logger ID.
   */
  const int32_t id_ = s_logger_id_++;

  /**
   * @brief Constructor for the logger.
   * @param bool Flag to indicate if the logger is a static instance.
   */
  serilog(bool) {
    s_initialized_ = false;
    s_terminating_ = false;
    base_level = logging_level::fatal;
    base_level_serilog = logging_level::verbose;
    _s_dispatch_interval = std::chrono::milliseconds(100);
    static_instance_ = true;
    register_logger(this);
  }

  /**
   * @brief Static method to handle sending log events in a loop.
   */
  static void send_events_loop_handler() {
    // std::cout << "starting serilog thread" << std::endl;
    while (!s_terminating_) {
      std::this_thread::sleep_for(_s_dispatch_interval);
      send_events_handler();
    }
    // std::cout << "ending serilog thread" << std::endl;
    s_thread_finished_.notify_all();
  }

  /**
   * @brief Generic logging function for a specific log level.
   * @param L Log level.
   * @param message Log message.
   * @param properties Log properties.
   */
  template <logging_level L>
  void instance_log_generic(std::string message, serilog_properties_vector_t &&properties) const {
    if (L < level && L < level_serilog)
      return;
    enqueue(std::move(message), make_context(L, properties));
  }

  /**
   * @brief Generic logging function for a specific log level without properties.
   * @param L Log level.
   * @param message Log message.
   */
  template <logging_level L> void instance_log_generic(std::string message) const {
    if (L < level && L < level_serilog)
      return;
    enqueue(std::move(message), make_context(L));
  }

  /**
   * @brief Start the log dispatch loop in a separate thread.
   */
  void start_thread() const {
    if (!static_instance_)
      return;
    std::thread t(&serilog::send_events_loop_handler);
    t.detach();
  }

  /**
   * @brief Get the shared logger instance.
   * @return serilog& The shared logger instance.
   */
  [[nodiscard]] static serilog &shared_instance() {
    static serilog instance(true);
    return instance;
  }

  /**
   * @brief Create a serilog_context with properties.
   * @param level_ Logging level.
   * @param properties Log properties.
   * @return serilog_context The created context.
   */
  serilog_context make_context(logging_level level_, serilog_properties_vector_t properties) const {
    auto ctx = serilog_context(level_, std::move(properties), name_);
    ctx.append(properties_);
    ctx.append(s_shared_properties_);
    if (!enrich_.empty()) {
      for (auto &enrich : enrich_) {
        enrich(ctx);
      }
    }
    if (!s_enrich_.empty()) {
      for (auto &enrich : s_enrich_) {
        enrich(ctx);
      }
    }
    return ctx;
  }

  /**
   * @brief Create a serilog_context without properties.
   * @param level_ Logging level.
   * @return serilog_context The created context.
   */
  serilog_context make_context(logging_level level_) const {
    auto ctx = serilog_context(level_, properties_, name_);
    ctx.append(s_shared_properties_);
    if (!enrich_.empty()) {
      for (auto &enrich : enrich_) {
        enrich(ctx);
      }
    }
    if (!s_enrich_.empty()) {
      for (auto &enrich : s_enrich_) {
        enrich(ctx);
      }
    }
    return ctx;
  }

  /**
   * @brief Enqueue a log entry.
   * @param message Log message.
   * @param context_ Log context.
   */
  void enqueue(std::string message, serilog_context &&context_) const {
    auto *entry = new serilog_log_entry(std::move(message), std::move(context_));

    if (entry->context.level >= level_serilog) {
      std::lock_guard<std::mutex> guard(logs_mutex_);
      serilog_dispatch_queue_.push_back(entry);
    }

    if (entry->context.level >= level && format_mode == CONSOLE) {
      static constexpr char esc_char = 27;
      std::stringstream ss;
      ss << entry->time << "\t" << entry->context.logger_name << "\t["
         << logging_level_strings_short[entry->context.level] << "]\t" << esc_char << "[1m" << entry->message()
         << esc_char << "[0m\t\t";
      if (!entry->context.empty()) {
        for (size_t i = 0; i < entry->context.size(); ++i) {
          ss << entry->context[i].first << "=" << entry->context[i].second.str_val << " ";
        }
      }
      ss << std::endl;
      if (entry->context.level > logging_level::warning) {
        std::cerr << ss.str();
        std::cerr.flush();
      } else {
        std::cout << ss.str();
        std::cout.flush();
      }
    }
  }

  /**
   * @brief Transfer logs from a queue to the serilog_dispatch_queue.
   * @param queue_ Queue containing log entries.
   */
  void transfer_logs(std::vector<serilog_log_entry *> &queue_) const {
    std::lock_guard<std::mutex> guard(logs_mutex_);
    serilog_dispatch_queue_.reserve(serilog_dispatch_queue_.size() + queue_.size());
    serilog_dispatch_queue_.insert(serilog_dispatch_queue_.end(), queue_.begin(), queue_.end());
  }

  /**
   * @brief Register a logger.
   * @param logger Logger to be registered.
   */
  static void register_logger(serilog *logger) {
    std::lock_guard<std::mutex> guard(s_loggers_mutex_);
    _s_loggers.push_back(logger);
  }

  /**
   * @brief Unregister a logger.
   * @param logger Logger to be unregistered.
   */
  static void unregister_logger(serilog *logger) {
    std::lock_guard<std::mutex> guard(s_loggers_mutex_);
    const auto pos =
        std::find_if(_s_loggers.begin(), _s_loggers.end(), [&](auto &l_logger) { return logger == l_logger; });

    if (pos != _s_loggers.end()) {
      // std::cout << "Delete logger found" << std::endl;
      _s_loggers.erase(pos);
    }
  }

  /**
   * @brief Initialize the logger with a name.
   * @param name Logger name.
   */
  void initialize(const char *name) {
    level = base_level;
    level_serilog = base_level_serilog;
    if (name != name_) {
      std::strcpy(name_, name);
    }

    register_logger(this);
    shared_instance().init(level, level_serilog);
  }
};
} // namespace API_COMMON_NAMESPACE::serilog
