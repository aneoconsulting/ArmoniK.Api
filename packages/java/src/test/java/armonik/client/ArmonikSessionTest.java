package armonik.client;

import armonik.api.grpc.v1.sessions.SessionsGrpc;
import armonik.client.session.impl.SesssionClientSync;
import io.grpc.ManagedChannel;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.when;


@ExtendWith(MockitoExtension.class)
class ArmonikSessionTest {

 // private SessionsGrpc.SessionsBlockingStub stub = Mockito.mock(SessionsGrpc.SessionsBlockingStub.class);

  @Mock
  private ManagedChannel mockChannel;

  @Mock
  private SessionsGrpc.SessionsBlockingStub sessionsBlockingStub;

  private SessionsGrpc sessionsGrpc = spy(SessionsGrpc.class);

  private SesssionClientSync armonikSession;

  @BeforeEach
  void setUp() {
    armonikSession = new SesssionClientSync(mockChannel);
    when(sessionsGrpc.newBlockingStub(mockChannel)).thenReturn(sessionsBlockingStub);
  }

  @Test
  void tes(){
    Assertions.assertEquals("1","1");
  }

//  @Test
//  void createSession() {
//    List<String> paritionsId = List.of("test");
//    Objects.TaskOptions taskOptions = Mockito.mock(Objects.TaskOptions.class);
//
//    when(mockSessionsStub.createSession(any()))
//      .thenReturn(SessionsCommon.CreateSessionReply.newBuilder()
//        .setSessionId("session-id")
//        .build()
//      );
//
//    Assertions.assertEquals("session-id", armonikSession.createSession(taskOptions, paritionsId));
//
//  }

}
