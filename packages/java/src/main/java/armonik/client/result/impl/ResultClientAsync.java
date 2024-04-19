package armonik.client.result.impl;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.results.ResultsCommon;
import armonik.api.grpc.v1.results.ResultsCommon.*;
import armonik.api.grpc.v1.results.ResultsFilters;
import armonik.api.grpc.v1.results.ResultsGrpc;
import armonik.api.grpc.v1.results.ResultsGrpc.ResultsStub;
import armonik.client.result.impl.util.factory.ResultClientRequestFactory;
import armonik.client.result.impl.util.records.DeleteResultsDataResponseRecord;
import armonik.client.result.impl.util.records.WatchResultResponseRecord;
import armonik.client.result.spec.IResultClientAsync;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

/**
 * ResultClientAsync is an asynchronous implementation of the {@link IResultClientAsync} interface.
 * It communicates with the result service using a non-blocking stub, making asynchronous calls to perform various result-related operations.
 */
public class ResultClientAsync implements IResultClientAsync {
  private final ResultsStub resultsStub;

  public ResultClientAsync(ManagedChannel managedChannel) {
    this.resultsStub = ResultsGrpc.newStub(managedChannel);
  }

  @Override
  public StreamObserver<WatchResultRequest> watchResults(StreamObserver<WatchResultResponseRecord> responseObserver) {
    StreamObserver<WatchResultResponse> proxyObserver = new StreamObserver<>() {
      @Override
      public void onNext(WatchResultResponse watchResultResponse) {
        responseObserver.onNext(new WatchResultResponseRecord(watchResultResponse.getStatus(), watchResultResponse.getResultIdsList()));
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    return resultsStub.watchResults(proxyObserver);
  }

  @Override
  public void getServiceConfiguration(StreamObserver<Integer> responseObserver) {
    StreamObserver<ResultsServiceConfigurationResponse> observer = new StreamObserver<ResultsServiceConfigurationResponse>() {
      @Override
      public void onNext(ResultsServiceConfigurationResponse resultsServiceConfigurationResponse) {
        responseObserver.onNext(resultsServiceConfigurationResponse.getDataChunkMaxSize());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    resultsStub.getServiceConfiguration(Objects.Empty.newBuilder().build(), observer);
  }


  @Override
  public void deleteResultsData(String sessionId, List<String> resultIds, StreamObserver<DeleteResultsDataResponseRecord> responseObserver) {
    StreamObserver<DeleteResultsDataResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(DeleteResultsDataResponse deleteResultsDataResponse) {
        responseObserver.onNext(new DeleteResultsDataResponseRecord(sessionId, deleteResultsDataResponse.getResultIdList()));
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    ResultsCommon.DeleteResultsDataRequest request = ResultClientRequestFactory.createDeleteResultsDataRequest(sessionId, resultIds);


    resultsStub.deleteResultsData(request, observer);
  }

  @Override
  public void downloadResultData(String sessionId, String resultId, StreamObserver<byte[]> responseObserver) {
    StreamObserver<DownloadResultDataResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(DownloadResultDataResponse downloadResultDataResponse) {
        responseObserver.onNext(downloadResultDataResponse.getDataChunk().toByteArray());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    DownloadResultDataRequest request = ResultClientRequestFactory.createDownloadResultDataRequest(sessionId, resultId);


    resultsStub.downloadResultData(request, observer);
  }


  @Override
  public StreamObserver<UploadResultDataRequest> uploadResultData(String sessionId, String resultId, String payload, StreamObserver<ResultRaw> responseObserver) {
    StreamObserver<UploadResultDataResponse> proxyObserver = new StreamObserver<>() {
      @Override
      public void onNext(UploadResultDataResponse uploadResultDataResponse) {
        responseObserver.onNext(uploadResultDataResponse.getResult());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };
    return resultsStub.uploadResultData(proxyObserver);

  }

  @Override
  public void createResults(CreateResultsRequest request, StreamObserver<Map<String, String>> responseObserver) {
    StreamObserver<CreateResultsResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CreateResultsResponse createResultsResponse) {
        responseObserver
          .onNext(
            createResultsResponse
              .getResultsList()
              .stream()
              .collect(Collectors.toMap(ResultRaw::getName, ResultRaw::getResultId))
          );
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    resultsStub.createResults(request, observer);
  }

  @Override
  public void createResultsMetaData(String sessionId, List<String> names, StreamObserver<List<ResultRaw>> responseObserver) {
    CreateResultsMetaDataRequest request = ResultClientRequestFactory.createCreateResultsMetaDataRequest(sessionId, names);

    StreamObserver<CreateResultsMetaDataResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CreateResultsMetaDataResponse createResultsMetaDataResponse) {
        responseObserver.onNext(createResultsMetaDataResponse.getResultsList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    resultsStub.createResultsMetaData(request, observer);
  }

  @Override
  public void getOwnerTaskId(String sessionId, List<String> resultIds, StreamObserver<Map<String, String>> responseObserver) {
    StreamObserver<GetOwnerTaskIdResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(GetOwnerTaskIdResponse getOwnerTaskIdResponse) {
        responseObserver.onNext(getOwnerTaskIdResponse.getResultTaskList()
          .stream()
          .collect(Collectors.toMap(GetOwnerTaskIdResponse.MapResultTask::getResultId, GetOwnerTaskIdResponse.MapResultTask::getTaskId)));
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    GetOwnerTaskIdRequest request = ResultClientRequestFactory.createGetOwnerTaskIdRequest(sessionId, resultIds);


    resultsStub.getOwnerTaskId(request, observer);
  }


  @Override
  public void getResult(String resultId, StreamObserver<ResultRaw> responseObserver) {
    StreamObserver<GetResultResponse> observer = new StreamObserver<GetResultResponse>() {
      @Override
      public void onNext(GetResultResponse getResultResponse) {
        responseObserver.onNext(getResultResponse.getResult());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    GetResultRequest request = ResultClientRequestFactory.createGetResultRequest(resultId);

    resultsStub.getResult(request, observer);
  }


  @Override
  public void listResults(ResultsFilters.Filters filters, int total, int page, int pageSize, ListResultsRequest.Sort sort, StreamObserver<List<ResultRaw>> responseObserver) {
    StreamObserver<ListResultsResponse> observer = new StreamObserver<ListResultsResponse>() {
      @Override
      public void onNext(ListResultsResponse listResultsResponse) {
        responseObserver.onNext(listResultsResponse.getResultsList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    ListResultsRequest request = ResultClientRequestFactory.createListResultsRequest(filters, total, page, pageSize, sort);

    resultsStub.listResults(request, observer);
  }
}
