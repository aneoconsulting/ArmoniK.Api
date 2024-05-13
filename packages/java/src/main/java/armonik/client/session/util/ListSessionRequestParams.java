package armonik.client.session.util;

import armonik.api.grpc.v1.sessions.SessionsCommon;
import armonik.api.grpc.v1.sessions.SessionsFilters;
/**
 * ListSessionRequestParams represents parameters for listing sessions.
 * It encapsulates information such as the page number, page size, sorting criteria, and filtering options.
 */
public record ListSessionRequestParams(int page, int pageSize, SessionsCommon.ListSessionsRequest.Sort sort, SessionsFilters.Filters filter) {
}
