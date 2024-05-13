package armonik.client.result;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsCommon.*;
import armonik.api.grpc.v1.results.ResultsGrpc;
import armonik.api.grpc.v1.results.ResultsGrpc.ResultsStub;
import armonik.client.result.util.factory.ResultClientRequestFactory;
import armonik.client.result.util.records.ListResultsRequestParams;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

@Deprecated(forRemoval = true)
public class ResultClientStream {
  private final ResultsStub resultsStub;

  public ResultClientStream(ManagedChannel managedChannel) {
    this.resultsStub = ResultsGrpc.newStub(managedChannel);
  }

  public StreamObserver<WatchResultRequest> watchResults(StreamObserver<WatchResultResponse> observer) {
    return resultsStub.watchResults(observer);
  }

  public void getServiceConfiguration(StreamObserver<ResultsServiceConfigurationResponse> observer) {
    resultsStub.getServiceConfiguration(Objects.Empty.newBuilder().build(), observer);
  }


  public void deleteResultsData(String sessionId, List<String> resultIds, StreamObserver<DeleteResultsDataResponse> observer) {
    DeleteResultsDataRequest request = ResultClientRequestFactory.createDeleteResultsDataRequest(sessionId, resultIds);
    resultsStub.deleteResultsData(request, observer);
  }

  public void downloadResultData(String sessionId, String resultId, StreamObserver<DownloadResultDataResponse> observer) {
    DownloadResultDataRequest request = ResultClientRequestFactory.createDownloadResultDataRequest(sessionId, resultId);
    resultsStub.downloadResultData(request, observer);
  }


  public StreamObserver<UploadResultDataRequest> uploadResultData(StreamObserver<UploadResultDataResponse> observer) {
    return resultsStub.uploadResultData(observer);
  }

  public void createResults(CreateResultsRequest request, StreamObserver<CreateResultsResponse> observer) {
    resultsStub.createResults(request, observer);
  }

  public void createResultsMetaData(String sessionId, List<String> names, StreamObserver<CreateResultsMetaDataResponse> observer) {
    CreateResultsMetaDataRequest request = ResultClientRequestFactory.createCreateResultsMetaDataRequest(sessionId, names);
    resultsStub.createResultsMetaData(request, observer);
  }

  public void getOwnerTaskId(String sessionId, List<String> resultIds, StreamObserver<GetOwnerTaskIdResponse> observer) {
    GetOwnerTaskIdRequest request = ResultClientRequestFactory.createGetOwnerTaskIdRequest(sessionId, resultIds);
    resultsStub.getOwnerTaskId(request, observer);
  }


  public void getResult(String resultId, StreamObserver<GetResultResponse> observer) {
    GetResultRequest request = ResultClientRequestFactory.createGetResultRequest(resultId);
    resultsStub.getResult(request, observer);
  }


  public void listResults(ListResultsRequestParams listResultsRequestParams, StreamObserver<ListResultsResponse> observer) {
    ListResultsRequest request = ResultClientRequestFactory
      .createListResultsRequest(
        listResultsRequestParams.filters(),
        listResultsRequestParams.total(),
        listResultsRequestParams.page(),
        listResultsRequestParams.pageSize(),
        listResultsRequestParams.sort()
      );
    resultsStub.listResults(request, observer);
  }
}


