#pragma once

#include "sessions_common.pb.h"
#include "sessions_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

/**
 * Session client wrapper
 */
class SessionsClient {
public:
  explicit SessionsClient(std::unique_ptr<armonik::api::grpc::v1::sessions::Sessions::StubInterface> stub)
      : stub(std::move(stub)){};

  /**
   * Create a new session
   * @param default_task_options Default task options for the session
   * @param partitions Partitions the session will be able to send tasks to
   * @return Session id
   */
  std::string create_session(armonik::api::grpc::v1::TaskOptions default_task_options,
                             const std::vector<std::string> &partitions = {});

  /**
   * Get informations about the given session
   * @param session_id Session id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw get_session(std::string session_id);

  /**
   * Cancel a session
   * @param session_id Session id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw cancel_session(std::string session_id);

  /**
   * List the Sessions
   * @param filters Filter to be used
   * @param total Output for the total of session available for this request (used for pagination)
   * @param page Page to request, use -1 to get all pages.
   * @param page_size Size of the requested page, ignored if page is -1
   * @param sort How the sessions are sorted, ascending creation date by default
   * @return List of sessions
   *
   * @note If the sessions corresponding to the filters change while this call is going for page==-1,
   * or between calls, then the returned values may not be consistent depending on the sorting used.
   * For example, a sort by ascending creation date (the default) will be stable if sessions are being created in
   * between requests.
   */
  std::vector<armonik::api::grpc::v1::sessions::SessionRaw>
  list_sessions(armonik::api::grpc::v1::sessions::Filters filters, int32_t &total, int32_t page = -1,
                int32_t page_size = 500,
                armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort sort = default_sort);

  /**
   * Pause a session
   *
   * @param session_id Session Id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw pause_session(std::string session_id);

  /**
   * Resume a session
   *
   * @param session_id Session Id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw resume_session(std::string session_id);

  /**
   * Purge a session
   *
   * @param session_id Session Id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw purge_session(std::string session_id);

  /**
   * Delete a session
   *
   * @param session_id Session Id
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw delete_session(std::string session_id);

  /**
   * Stop a new tasks submission in a session
   *
   * @param session_id Session Id
   * @param client boolean to stop client's task submission
   * @param worker boolean to stop worker's task submissions
   * @return SessionRaw object containing information about the session
   */
  armonik::api::grpc::v1::sessions::SessionRaw stop_submission_session(std::string session_id, bool client = true,
                                                                       bool worker = true);

private:
  std::unique_ptr<armonik::api::grpc::v1::sessions::Sessions::StubInterface> stub;
  static const armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort default_sort;
};

} // namespace client
} // namespace api
} // namespace armonik
