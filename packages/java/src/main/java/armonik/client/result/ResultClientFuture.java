package armonik.client.result;

import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest.Sort;
import armonik.api.grpc.v1.results.ResultsCommon.ResultRaw;
import armonik.api.grpc.v1.results.ResultsFilters;
import armonik.api.grpc.v1.results.ResultsFilters.Filters;
import armonik.client.result.util.records.DeleteResultsDataResponseRecord;
import io.grpc.ManagedChannel;

import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

/**
 * ResultClientFuture provides asynchronous operations for interacting with result-related functionalities.
 * It utilizes CompletableFuture to asynchronously perform operations using the ResultClient.
 */
public class ResultClientFuture {
  /** The ResultClient used for synchronous communication with the server. */
  private final ResultClient resultClient;

  /**
   * Constructs a new ResultClientFuture with the specified managed channel.
   *
   * @param channel the managed channel used for communication with the server
   */
  public ResultClientFuture(ManagedChannel channel) {
    this.resultClient = new ResultClient(channel);
  }

  /**
   * Asynchronously retrieves the service configuration data chunk max size from the server.
   *
   * @return a CompletableFuture representing the asynchronous operation to retrieve the service configuration
   */
  public CompletableFuture<Integer> getServiceConfiguration() {
    return CompletableFuture.supplyAsync(resultClient::getServiceConfiguration);
  }

  /**
   * Asynchronously deletes result data for the specified session ID and result IDs.
   *
   * @param sessionId the session ID associated with the results to delete
   * @param resultIds the list of result IDs to delete
   * @return a CompletableFuture representing the asynchronous operation to delete result data
   */
  public CompletableFuture<DeleteResultsDataResponseRecord> deleteResultsData(String sessionId, List<String> resultIds) {
    return CompletableFuture.supplyAsync(() -> resultClient.deleteResultsData(sessionId, resultIds)) ;
  }

  /**
   * Asynchronously downloads result data for the specified session ID and result ID.
   *
   * @param sessionId the session ID associated with the result to download
   * @param resultId the result ID to download
   * @return a CompletableFuture representing the asynchronous operation to download result data
   */
  public CompletableFuture<List<byte[]>> downloadResultData(String sessionId, String resultId) {
    return CompletableFuture.supplyAsync(() -> resultClient.downloadResultData(sessionId, resultId)) ;
  }


  /**
   * Asynchronously creates results based on the specified request.
   *
   * @param request the request containing the data to create results
   * @return a CompletableFuture representing the asynchronous operation to create results
   */
  public CompletableFuture<List<ResultRaw>> createResults(ResultsCommon.CreateResultsRequest request) {
    return CompletableFuture.supplyAsync(() -> resultClient.createResults(request)) ;
  }

  /**
   * Asynchronously creates metadata for results associated with the specified session ID and names.
   *
   * @param sessionId the session ID for which metadata is being created
   * @param names the list of names for the results
   * @return a CompletableFuture representing the asynchronous operation to create results metadata
   */
  public  CompletableFuture<List<ResultRaw>> createResultsMetaData(String sessionId, List<String> names) {
    return CompletableFuture.supplyAsync(() -> resultClient.createResultsMetaData(sessionId, names)) ;
  }

  /**
   * Asynchronously retrieves a map of result IDs to task IDs for the specified session ID and result IDs.
   *
   * @param sessionId the session ID associated with the results
   * @param resultIds the list of result IDs for which task IDs are requested
   * @return a CompletableFuture representing the asynchronous operation to retrieve result task IDs
   */
  public CompletableFuture<Map<String, String>> getOwnerTaskId(String sessionId, List<String> resultIds) {
    return CompletableFuture.supplyAsync(() -> resultClient.getOwnerTaskId(sessionId, resultIds)) ;
  }

  /**
   * Asynchronously retrieves the result with the specified result ID.
   *
   * @param resultId the ID of the result to retrieve
   * @return a CompletableFuture representing the asynchronous operation to retrieve the result
   */
  public  CompletableFuture<ResultRaw> getResult(String resultId) {
    return CompletableFuture.supplyAsync(() -> resultClient.getResult(resultId)) ;
  }

  /**
   * Asynchronously lists results based on the specified filters, total count, pagination parameters, and sorting criteria.
   *
   * @param filters the filters to apply to the result list
   * @param total the total count of results
   * @param page the page number of the results to retrieve
   * @param pageSize the size of each page of results
   * @param sort the sorting criteria for the results
   * @return a CompletableFuture representing the asynchronous operation to list results
   */
  public CompletableFuture<List<ResultRaw>>  listResults(Filters filters, int total, int page, int pageSize, Sort sort) {
    return CompletableFuture.supplyAsync(() -> resultClient.listResults(filters, total, page, pageSize, sort)) ;
  }
}
