package armonik.client.result.util.records;

import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsFilters;

public record ListResultsRequestParams(ResultsFilters.Filters filters, int total, int page, int pageSize, ResultsCommon.ListResultsRequest.Sort sort) {
}
