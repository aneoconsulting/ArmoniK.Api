package armonik.client.result.util.factory;

import armonik.api.grpc.v1.results.ResultsCommon.*;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest.Sort;
import armonik.api.grpc.v1.results.ResultsFilters.Filters;

import java.util.List;

/**
 * ResultClientRequestFactory provides static methods for creating gRPC request objects related to result management.
 * It encapsulates the logic for constructing various types of result-related requests, such as deleting results data,
 * downloading result data, creating results metadata, retrieving owner task IDs, retrieving result details, and listing results.
 */
public abstract class ResultClientRequestFactory {

  public static DeleteResultsDataRequest createDeleteResultsDataRequest(String sessionId, List<String> resultIds) {
    return DeleteResultsDataRequest.newBuilder()
      .setSessionId(sessionId)
      .addAllResultId(resultIds)
      .build();
  }

  public static DownloadResultDataRequest createDownloadResultDataRequest(String sessionId, String resultId) {
    return DownloadResultDataRequest.newBuilder()
      .setSessionId(sessionId)
      .setResultId(resultId)
      .build();
  }


  public static CreateResultsMetaDataRequest createCreateResultsMetaDataRequest(String sessionId, List<String> names) {
    return  CreateResultsMetaDataRequest.newBuilder()
      .setSessionId(sessionId)
      .addAllResults(names.stream().map(name -> CreateResultsMetaDataRequest.ResultCreate.newBuilder().setName(name).build()).toList())
      .build();

  }

  public static GetOwnerTaskIdRequest createGetOwnerTaskIdRequest(String sessionId, List<String> resultIds) {
    return GetOwnerTaskIdRequest.newBuilder()
      .setSessionId(sessionId)
      .addAllResultId(resultIds)
      .build();

  }

  public static GetResultRequest createGetResultRequest(String resultId) {
    return  GetResultRequest.newBuilder()
      .setResultId(resultId)
      .build();
  }

  public static ListResultsRequest createListResultsRequest(Filters filters, int total, int page, int pageSize, Sort sort) {
    return ListResultsRequest.newBuilder()
      .setFilters(filters)
      .setSort(sort)
      .setPage(page)
      .setPageSize(pageSize)
      .build();


  }


}
