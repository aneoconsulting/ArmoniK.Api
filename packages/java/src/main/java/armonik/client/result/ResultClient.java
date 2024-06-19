package armonik.client.result;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

import com.google.protobuf.ByteString;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.results.ResultsCommon.CreateResultsMetaDataRequest;
import armonik.api.grpc.v1.results.ResultsCommon.CreateResultsRequest;
import armonik.api.grpc.v1.results.ResultsCommon.DownloadResultDataRequest;
import armonik.api.grpc.v1.results.ResultsCommon.DownloadResultDataResponse;
import armonik.api.grpc.v1.results.ResultsCommon.GetOwnerTaskIdRequest;
import armonik.api.grpc.v1.results.ResultsCommon.GetOwnerTaskIdResponse.MapResultTask;
import armonik.api.grpc.v1.results.ResultsCommon.GetResultRequest;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest.Sort;
import armonik.api.grpc.v1.results.ResultsCommon.ResultRaw;
import armonik.api.grpc.v1.results.ResultsFilters.Filters;
import armonik.api.grpc.v1.results.ResultsGrpc;
import io.grpc.ManagedChannel;

/**
 * ResultClient provides methods for interacting with result-related
 * functionalities.
 * It communicates with a gRPC server using a blocking stub to perform various
 * operations on results.
 */
public class ResultClient {
  /** The blocking stub for communicating with the gRPC server. */
  private final ResultsGrpc.ResultsBlockingStub resultsBlockingStub;

  /**
   * Constructs a new ResultClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the
   *                       server
   */
  public ResultClient(ManagedChannel managedChannel) {
    this.resultsBlockingStub = ResultsGrpc.newBlockingStub(managedChannel);
  }

  /**
   * Retrieves the service configuration data chunk max size from the server.
   *
   * @return the service configuration data chunk max size
   */
  public int getServiceConfiguration() {
    return resultsBlockingStub.getServiceConfiguration(Objects.Empty.newBuilder().build()).getDataChunkMaxSize();
  }

  /**
   * Downloads result data for the specified session ID and result ID.
   *
   * @param sessionId the session ID associated with the result to download
   * @param resultId  the result ID to download
   * @return a list of byte arrays representing the downloaded data chunks
   */
  public List<byte[]> downloadResultData(String sessionId, String resultId) {
    DownloadResultDataRequest request = DownloadResultDataRequest.newBuilder()
        .setSessionId(sessionId)
        .setResultId(resultId)
        .build();
    Iterator<DownloadResultDataResponse> iterator = resultsBlockingStub.downloadResultData(request);
    List<DownloadResultDataResponse> list = new ArrayList<>();

    iterator.forEachRemaining(list::add);

    return list.stream()
        .map(DownloadResultDataResponse::getDataChunk)
        .map(ByteString::toByteArray)
        .toList();
  }

  /**
   * Creates results based on the specified request.
   *
   * @param request the request containing the data to create results
   * @return a list of ResultRaw objects representing the created results
   */
  public List<ResultRaw> createResults(CreateResultsRequest request) {
    return resultsBlockingStub.createResults(request).getResultsList();

  }

  /**
   * Creates metadata for results associated with the specified session ID and
   * names.
   *
   * @param sessionId the session ID for which metadata is being created
   * @param names     the list of names for the results
   * @return a list of ResultRaw objects representing the created metadata
   */
  public List<ResultRaw> createResultsMetaData(String sessionId, List<String> names) {
    CreateResultsMetaDataRequest request = CreateResultsMetaDataRequest.newBuilder()
        .setSessionId(sessionId)
        .addAllResults(names.stream()
            .map(name -> CreateResultsMetaDataRequest.ResultCreate.newBuilder().setName(name).build()).toList())
        .build();

    return resultsBlockingStub.createResultsMetaData(request).getResultsList();
  }

  /**
   * Retrieves a map of result IDs to task IDs for the specified session ID and
   * result IDs.
   *
   * @param sessionId the session ID associated with the results
   * @param resultIds the list of result IDs for which task IDs are requested
   * @return a map where result IDs are mapped to their corresponding task IDs
   */
  public Map<String, String> getOwnerTaskId(String sessionId, List<String> resultIds) {
    GetOwnerTaskIdRequest request = GetOwnerTaskIdRequest.newBuilder()
        .setSessionId(sessionId)
        .addAllResultId(resultIds)
        .build();
    return resultsBlockingStub.getOwnerTaskId(request).getResultTaskList()
        .stream()
        .collect(Collectors.toMap(MapResultTask::getResultId, MapResultTask::getTaskId));
  }

  /**
   * Retrieves the result with the specified result ID.
   *
   * @param resultId the ID of the result to retrieve
   * @return the ResultRaw object representing the retrieved result
   */
  public ResultRaw getResult(String resultId) {
    GetResultRequest request = GetResultRequest.newBuilder()
        .setResultId(resultId)
        .build();
    return resultsBlockingStub.getResult(request).getResult();
  }

  /**
   * Lists results based on the specified filters, total count, pagination
   * parameters, and sorting criteria.
   *
   * @param filters  the filters to apply to the result list
   * @param total    the total count of results
   * @param page     the page number of the results to retrieve
   * @param pageSize the size of each page of results
   * @param sort     the sorting criteria for the results
   * @return a list of ResultRaw objects representing the retrieved results
   */
  public List<ResultRaw> listResults(Filters filters, int total, int page, int pageSize, Sort sort) {
    ListResultsRequest request = ListResultsRequest.newBuilder()
        .setFilters(filters)
        .setSort(sort)
        .setPage(page)
        .setPageSize(pageSize)
        .build();
    return resultsBlockingStub.listResults(request).getResultsList();
  }
}
