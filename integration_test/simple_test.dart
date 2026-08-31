import 'package:flutter_test/flutter_test.dart';
import 'package:e2xf/main.dart';
import 'package:e2xf/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';
import 'package:e2xf/main_viewmodel.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('Shows the conversion form', (WidgetTester tester) async {
    await tester.pumpWidget(MainApp(viewModel: MainViewModel()));
    await tester.pumpAndSettle();

    expect(find.text('选择Excel文件'), findsOneWidget);
    expect(find.text('选择模块文件'), findsOneWidget);
    expect(find.text('开始转换'), findsOneWidget);
  });
}
