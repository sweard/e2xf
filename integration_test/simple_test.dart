import 'package:flutter_test/flutter_test.dart';
import 'package:e2xf/main.dart';
import 'package:e2xf/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';
import 'package:e2xf/main_viewmodel.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('Can call rust function', (WidgetTester tester) async {
    await tester.pumpWidget(MainApp(viewModel: MainViewModel()));
    expect(find.textContaining('Result: `Hello, Tom!`'), findsOneWidget);
  });
}
