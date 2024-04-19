package armonik.client.result.impl;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.results.ResultsCommon.*;
import armonik.api.grpc.v1.results.ResultsCommon.GetOwnerTaskIdResponse.MapResultTask;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest.Sort;
import armonik.api.grpc.v1.results.ResultsGrpc;
import armonik.client.result.impl.util.factory.ResultClientRequestFactory;
import armonik.client.result.impl.util.records.DeleteResultsDataResponseRecord;
import armonik.client.result.spec.IResultClientSync;
import io.grpc.ManagedChannel;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

import static armonik.api.grpc.v1.results.ResultsFilters.*;

/**
 * ResultClientSync is a synchronous implementation of the {@link IResultClientSync} interface.
 * It communicates with the result service using a blocking stub, making synchronous calls to interact with result data.
 */
public class ResultClientSync implements IResultClientSync {
  private final ResultsGrpc.ResultsBlockingStub resultsBlockingStub;

  public ResultClientSync(ManagedChannel managedChannel) {
    this.resultsBlockingStub = ResultsGrpc.newBlockingStub(managedChannel);
  }

  @Override
  public int getServiceConfiguration() {
    return resultsBlockingStub.getServiceConfiguration(Objects.Empty.newBuilder().build()).getDataChunkMaxSize();
  }

  @Override
  public DeleteResultsDataResponseRecord deleteResultsData(String sessionId, List<String> resultIds) {
    DeleteResultsDataRequest request = ResultClientRequestFactory.createDeleteResultsDataRequest(sessionId, resultIds);
    return new DeleteResultsDataResponseRecord(sessionId, resultsBlockingStub.deleteResultsData(request).getResultIdList());
  }


  @Override
  public List<byte[]> downloadResultData(String sessionId, String resultId) {
    DownloadResultDataRequest request = ResultClientRequestFactory.createDownloadResultDataRequest(sessionId, resultId);
    Iterator<DownloadResultDataResponse> iterator = resultsBlockingStub.downloadResultData(request);
    List<DownloadResultDataResponse> list = new ArrayList<>();

    iterator.forEachRemaining(list::add);
    return list.stream().map(downloadResultDataResponse -> downloadResultDataResponse.getDataChunk().toByteArray())
      .toList();
  }

  @Override
  public Map<String, String> createResults(CreateResultsRequest request) {
     return resultsBlockingStub.createResults(request).getResultsList()
       .stream()
       .collect(Collectors.toMap(ResultRaw::getName,ResultRaw::getResultId));
  }


  @Override
  public List<ResultRaw> createResultsMetaData(String sessionId, List<String> names) {
    CreateResultsMetaDataRequest request = ResultClientRequestFactory.createCreateResultsMetaDataRequest(sessionId, names);
    return resultsBlockingStub.createResultsMetaData(request).getResultsList();
  }

  @Override
  public Map<String, String> getOwnerTaskId(String sessionId, List<String> resultIds) {
    GetOwnerTaskIdRequest request = ResultClientRequestFactory.createGetOwnerTaskIdRequest(sessionId, resultIds);
    return resultsBlockingStub.getOwnerTaskId(request).getResultTaskList()
      .stream()
      .collect(Collectors.toMap(MapResultTask::getResultId, MapResultTask::getTaskId));
  }

  @Override
  public ResultRaw getResult(String resultId) {
    GetResultRequest request = ResultClientRequestFactory.createGetResultRequest(resultId);
    return resultsBlockingStub.getResult(request).getResult();
  }

  @Override
  public List<ResultRaw> listResults(Filters filters, int total, int page, int pageSize, Sort sort) {
    ListResultsRequest request = ResultClientRequestFactory.createListResultsRequest(filters, total, page, pageSize, sort);
    return resultsBlockingStub.listResults(request).getResultsList();
  }
}
