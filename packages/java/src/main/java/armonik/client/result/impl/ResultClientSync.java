package armonik.client.result.impl;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.results.ResultsCommon.*;
import armonik.api.grpc.v1.results.ResultsCommon.GetOwnerTaskIdResponse.MapResultTask;
import armonik.api.grpc.v1.results.ResultsCommon.ListResultsRequest.Sort;
import armonik.api.grpc.v1.results.ResultsGrpc;
import armonik.client.result.impl.util.factory.ResultClientRequestFactory;
import armonik.client.result.impl.util.records.SessionDeletedResultIds;
import armonik.client.result.spec.IResultClientSync;
import io.grpc.ManagedChannel;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

import static armonik.api.grpc.v1.results.ResultsFilters.*;

public class ResultClientSync implements IResultClientSync {
  private final ResultsGrpc.ResultsBlockingStub blockingStub;

  public ResultClientSync(ManagedChannel managedChannel) {
    this.blockingStub = ResultsGrpc.newBlockingStub(managedChannel);
  }

  @Override
  public int getServiceConfiguration() {
    return blockingStub.getServiceConfiguration(Objects.Empty.newBuilder().build()).getDataChunkMaxSize();
  }

  @Override
  public SessionDeletedResultIds deleteResultsData(String sessionId, List<String> resultIds) {
    DeleteResultsDataRequest request = ResultClientRequestFactory.createDeleteResultsDataRequest(sessionId, resultIds);
    return new SessionDeletedResultIds(sessionId, blockingStub.deleteResultsData(request).getResultIdList());
  }


  @Override
  public List<byte[]> downloadResultData(String sessionId, String resultId) {
    DownloadResultDataRequest request = ResultClientRequestFactory.createDownloadResultDataRequest(sessionId, resultId);
    Iterator<DownloadResultDataResponse> iterator = blockingStub.downloadResultData(request);
    List<DownloadResultDataResponse> list = new ArrayList<>();

    iterator.forEachRemaining(list::add);
    return list.stream().map(downloadResultDataResponse -> downloadResultDataResponse.getDataChunk().toByteArray())
      .toList();
  }

  @Override
  public Map<String, String> createResults(CreateResultsRequest request) {
     return blockingStub.createResults(request).getResultsList()
       .stream()
       .collect(Collectors.toMap(ResultRaw::getName,ResultRaw::getResultId));
  }


  @Override
  public List<ResultRaw> createResultsMetaData(String sessionId, List<String> names) {
    CreateResultsMetaDataRequest request = ResultClientRequestFactory.createCreateResultsMetaDataRequest(sessionId, names);
    return blockingStub.createResultsMetaData(request).getResultsList();
  }

  @Override
  public Map<String, String> getOwnerTaskId(String sessionId, List<String> resultIds) {
    GetOwnerTaskIdRequest request = ResultClientRequestFactory.createGetOwnerTaskIdRequest(sessionId, resultIds);
    return blockingStub.getOwnerTaskId(request).getResultTaskList()
      .stream()
      .collect(Collectors.toMap(MapResultTask::getResultId, MapResultTask::getTaskId));
  }

  @Override
  public ResultRaw getResult(String resultId) {
    GetResultRequest request = ResultClientRequestFactory.createGetResultRequest(resultId);
    return blockingStub.getResult(request).getResult();
  }

  @Override
  public List<ResultRaw> listResults(Filters filters, int total, int page, int pageSize, Sort sort) {
    ListResultsRequest request = ResultClientRequestFactory.createListResultsRequest(filters, total, page, pageSize, sort);
    return blockingStub.listResults(request).getResultsList();
  }
}
